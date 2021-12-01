use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Error;
use libcnb::data::layer_content_metadata::LayerContentMetadata;
use libcnb::layer_lifecycle::{LayerLifecycle, ValidateResult};
use libcnb::{get, get_and_extract, BuildContext, GenericPlatform};
use std::env;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Output};

use crate::util::config::{SFDXRuntimeConfig, SFPackageBuildpackConfig};

pub struct SFDXLayerLifecycle;

impl
    LayerLifecycle<
        GenericPlatform,
        SFPackageBuildpackConfig,
        SFPackageBuildpackConfig,
        HashMap<String, String>,
        anyhow::Error,
    > for SFDXLayerLifecycle
{
    fn create(
        &self,
        layer_path: &Path,
        build_context: &BuildContext<GenericPlatform, SFPackageBuildpackConfig>,
    ) -> Result<LayerContentMetadata<SFPackageBuildpackConfig>, anyhow::Error> {
        let runtime_sha256 = get_and_extract(
            &build_context.buildpack_descriptor.metadata.runtime.url,
            &layer_path,
            Some("sfdx/"),
        )?;

        Ok(LayerContentMetadata::default()
            .build(false)
            .cache(true)
            .launch(true)
            .metadata(SFPackageBuildpackConfig {
                runtime: SFDXRuntimeConfig {
                    url: build_context
                        .buildpack_descriptor
                        .metadata
                        .runtime
                        .url
                        .clone(),
                    manifest: build_context
                        .buildpack_descriptor
                        .metadata
                        .runtime
                        .manifest
                        .clone(),
                    sha256: runtime_sha256.clone(),
                },
            }))
    }

    fn validate(
        &self,
        _layer_path: &Path,
        layer_content_metadata: &LayerContentMetadata<SFPackageBuildpackConfig>,
        build_context: &BuildContext<GenericPlatform, SFPackageBuildpackConfig>,
    ) -> ValidateResult {
        // Get most recent sfdx.tar.xz manifest
        let manifest_content =
            get(&build_context.buildpack_descriptor.metadata.runtime.manifest).unwrap();
        let manifest = json::parse(&manifest_content).unwrap();

        let manifest_sha256 = match &manifest["sha256xz"] {
            json::JsonValue::String(s) => s,
            _ => "",
        };

        if layer_content_metadata.metadata.runtime.sha256 == manifest_sha256 {
            ValidateResult::KeepLayer
        } else {
            ValidateResult::RecreateLayer
        }
    }

    fn layer_lifecycle_data(
        &self,
        layer_path: &Path,
        _layer_content_metadata: LayerContentMetadata<SFPackageBuildpackConfig>,
    ) -> Result<HashMap<String, String>, Error> {
        let mut layer_env: HashMap<String, String> = HashMap::new();
        let bin_path = format!("{}/bin", &layer_path.to_str().unwrap());

        layer_env.insert(
            String::from("PATH"),
            format!(
                "{}:{}:{}",
                layer_path.join("bin").as_path().to_str().unwrap(),
                bin_path,
                env::var("PATH").unwrap_or(String::new()),
            ),
        );

        Ok(layer_env)
    }
}

pub fn sfdx_check_org(app_dir: &PathBuf, scratch_org_alias: &str) -> Result<bool, anyhow::Error> {
    let mut cmd = Command::new("sfdx");
    let result = cmd
        .current_dir(app_dir)
        .args(vec!["force:org:display", "-u", scratch_org_alias])
        .output();

    if let Ok(output) = result {
        if let Some(code) = output.status.code() {
            if !output.stdout.is_empty() {
                println!("{}", String::from_utf8(output.stdout)?);
            }
            if !output.stderr.is_empty() {
                println!("{}", String::from_utf8(output.stderr)?);
            }

            return Ok(code == 0);
        }
    }
    Ok(false)
}

pub fn sfdx_create_org(
    app_dir: &PathBuf,
    devhub_alias: &str,
    scratch_org_def_path: &str,
    scratch_org_duration: i32,
    scratch_org_alias: &str,
) -> Result<Output, anyhow::Error> {
    let mut cmd = Command::new("sfdx");
    cmd.current_dir(app_dir)
        .arg("force:org:create")
        .arg("-v")
        .arg(devhub_alias)
        .arg("-f")
        .arg(scratch_org_def_path)
        .arg("-d")
        .arg(scratch_org_duration.to_string())
        .arg("-a")
        .arg(scratch_org_alias);
    match cmd.output() {
        Ok(output) => {
            let status = output.status.code().unwrap();
            let stderr = String::from_utf8(output.stderr.to_owned()).unwrap();
            if status != 0 {
                Err(anyhow::anyhow!(
                    "failed to create scratch org on {} from {}:\n{}",
                    devhub_alias,
                    scratch_org_def_path,
                    stderr
                ))
            } else {
                Ok(output)
            }
        }
        Err(e) => Err(anyhow::anyhow!(
            "failed to create scratch org on {} from {} due to {}",
            devhub_alias,
            scratch_org_def_path,
            e
        )),
    }
}

pub fn sfdx_delete_org(
    app_dir: &PathBuf,
    devhub_alias: &str,
    scratch_org_alias: &str,
) -> Result<Output, anyhow::Error> {
    let mut cmd = Command::new("sfdx");
    cmd.current_dir(app_dir)
        .arg("force:org:delete")
        .arg("-v")
        .arg(devhub_alias)
        .arg("-u")
        .arg(scratch_org_alias)
        .arg("-p");
    match cmd.output() {
        Ok(output) => {
            let status = output.status.code().unwrap();
            let stderr = String::from_utf8(output.stderr.to_owned()).unwrap();
            if status != 0 {
                return Err(anyhow::anyhow!(
                    "failed to delete scratch org on {} named {}:\n {}",
                    devhub_alias,
                    scratch_org_alias,
                    stderr
                ));
            }
            Ok(output)
        }
        Err(e) => {
            eprintln!(
                "failed to delete scratch org on {} named {}",
                devhub_alias, scratch_org_alias
            );
            Err(anyhow::anyhow!(e))
        }
    }
}

pub fn sfdx_push_source(
    app_dir: &PathBuf,
    scratch_org_alias: &str,
    wait_seconds: i32,
) -> Result<Output, anyhow::Error> {
    let mut cmd = Command::new("sfdx");
    let mut child = cmd
        .current_dir(app_dir)
        .arg("force:source:push")
        .arg("-u")
        .arg(scratch_org_alias)
        .arg("-w")
        .arg(wait_seconds.to_string())
        .spawn()
        .expect("failed to execute command");

    output_stderr(&mut child);

    let output = child.wait_with_output().expect("failed to wait on child");
    if output.status.success() {
        Ok(output)
    } else {
        Err(anyhow::anyhow!(
            "failed to push source to {}:\n Exited with {}",
            scratch_org_alias,
            output.status.code().unwrap()
        ))
    }
}

pub struct SfdxResponse<R> {
    pub status: u8,
    pub result: R,
}

pub struct CreatePackageResult {
    pub created: bool,
    pub package_id: String,
}

pub struct FindPackageResult {
    pub package_id: String,
}

pub fn sfdx_find_package(
    app_dir: &PathBuf,
    devhub_alias: &String,
    package_name: &String,
) -> Result<SfdxResponse<FindPackageResult>, anyhow::Error> {
    let mut cmd = Command::new("sfdx");
    let output = cmd
        .current_dir(app_dir)
        .arg("force:package:list")
        .arg("--json")
        .arg("-v")
        .arg(devhub_alias)
        .output()
        .expect("failed to execute command");

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        let v: serde_json::Value = serde_json::from_str(stdout.as_str())?;
        let package_values = v["result"].as_array().unwrap();
        match package_values
            .iter()
            .find(|v| v["Name"].as_str().unwrap().eq(package_name))
        {
            Some(package) => Ok(SfdxResponse {
                status: 0,
                result: FindPackageResult {
                    package_id: package["Id"].as_str().unwrap().to_string(),
                },
            }),
            None => Ok(SfdxResponse {
                status: 1,
                result: FindPackageResult {
                    package_id: "".to_string(),
                },
            }),
        }
    } else {
        Err(anyhow::anyhow!(
            "failed to create new package {}:\n Exited with {}",
            package_name,
            output.status.code().unwrap()
        ))
    }
}

pub fn sfdx_create_package(
    app_dir: &PathBuf,
    devhub_alias: &String,
    package_name: &String,
    package_desc: &String,
    package_type: &String,
    package_root: &String,
) -> Result<SfdxResponse<CreatePackageResult>, anyhow::Error> {
    let mut cmd = Command::new("sfdx");
    let mut child = cmd
        .current_dir(app_dir)
        .arg("force:package:create")
        .arg("--json")
        .arg("-v")
        .arg(devhub_alias)
        .arg("-n")
        .arg(package_name)
        .arg("-d")
        .arg(package_desc)
        .arg("-t")
        .arg(package_type)
        .arg("-r")
        .arg(package_root)
        .spawn()
        .expect("failed to execute command");

    output_stderr(&mut child);

    let output = child.wait_with_output().expect("failed to wait on command");
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        let v: serde_json::Value = serde_json::from_str(stdout.as_str())?;
        let status = 0;
        let result = CreatePackageResult {
            created: true,
            package_id: v["result"]["Id"].to_string(),
        };
        Ok(SfdxResponse { status, result })
    } else {
        Err(anyhow::anyhow!(
            "failed to create new package {}:\n Exited with {}",
            package_name,
            output.status.code().unwrap()
        ))
    }
}

fn output_stderr(child: &mut Child) {
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        reader
            .lines()
            .filter_map(|line| line.ok())
            .for_each(|line| eprintln!("{}", line));
    }
}

pub fn sfdx_create_package_version(
    app_dir: &PathBuf,
    devhub_alias: &String,
    package_id: &String,
    org_def_path: &String,
    version_name: &String,
    version_number: &String,
    installation_key: &String,
    wait_seconds: i32,
) -> Result<serde_json::Value, anyhow::Error> {
    let mut cmd = Command::new("sfdx");
    cmd.current_dir(&app_dir)
        .arg("force:package:version:create")
        .arg("--json")
        .arg("-p")
        .arg(package_id)
        .arg("-v")
        .arg(devhub_alias)
        .arg("-f")
        .arg(org_def_path)
        .arg("-a")
        .arg(version_name)
        .arg("-n")
        .arg(version_number)
        .arg("-w")
        .arg(wait_seconds.to_string());
    if installation_key.is_empty() {
        cmd.arg("-x");
    } else {
        cmd.arg("-k").arg(installation_key);
    }
    let output = cmd.output().expect("failed to execute command");

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        let v: serde_json::Value = serde_json::from_str(stdout.as_str())?;
        Ok(v)
    } else {
        Err(anyhow::anyhow!(
            "failed to create new package version of {}:\n Exited with {}",
            package_id,
            output.status.code().unwrap()
        ))
    }
}

pub fn sfdx_test_apex(
    app_dir: &PathBuf,
    scratch_org_alias: &str,
    results_path: PathBuf,
    wait_seconds: i32,
) -> Result<Output, anyhow::Error> {
    let mut cmd = Command::new("sfdx");
    cmd.current_dir(app_dir)
        .arg("force:apex:test:run")
        .arg("-u")
        .arg(scratch_org_alias)
        .arg("-l")
        .arg("RunLocalTests")
        .arg("-w")
        .arg(wait_seconds.to_string())
        .arg("-r")
        .arg("tap")
        .arg("-d")
        .arg(results_path.as_os_str())
        .arg("-c")
        .arg("-v");

    match cmd.output() {
        Ok(output) => {
            let status = output.status.code().unwrap();
            let stderr = String::from_utf8(output.stderr.to_owned()).unwrap();
            // This is a Hack, to work around the platform bug that throws an error when no apex tests exist.
            if status != 0
                && !stderr
                    .contains("Always provide a classes, suites, tests, or testLevel property")
            {
                return Err(anyhow::anyhow!(
                    "failed to run apex tests on {}:\n {}",
                    scratch_org_alias,
                    stderr
                ));
            }
            Ok(output)
        }
        Err(e) => {
            eprintln!("failed to run apex tests on {}", scratch_org_alias);
            Err(anyhow::anyhow!(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use libcnb::data::buildpack::BuildpackToml;
    use libcnb::data::buildpack_plan::BuildpackPlan;
    use libcnb::{BuildContext, GenericPlatform, Platform};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn _setup_context(tmp_dir: &TempDir) -> BuildContext<GenericPlatform, toml::value::Table> {
        let app_dir = tmp_dir.path().join("app");
        let buildpack_dir = tmp_dir.path().join("buildpack");
        let layers_dir = tmp_dir.path().join("layers");
        let platform_env = tmp_dir.path().join("platform").join("env");

        for path in [&app_dir, &buildpack_dir, &layers_dir, &platform_env].iter() {
            fs::create_dir_all(path).unwrap();
        }
        let buildpack_toml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("buildpack.toml");
        fs::copy(&buildpack_toml_path, buildpack_dir.join("buildpack.toml")).unwrap();

        let stack_id = String::from("heroku-20");
        let platform = GenericPlatform::from_path(tmp_dir.path().join("platform")).unwrap();
        let buildpack_plan = BuildpackPlan {
            entries: Vec::new(),
        };
        let buildpack_descriptor: BuildpackToml<toml::value::Table> =
            toml::from_str(&fs::read_to_string(&buildpack_toml_path).unwrap()).unwrap();

        BuildContext {
            layers_dir,
            app_dir,
            buildpack_dir,
            stack_id,
            platform,
            buildpack_plan,
            buildpack_descriptor,
        }
    }

    #[test]
    fn test_if_runtime_exists_and_checksum_match() {
        // TODO need mocking to avoid outbound calls
    }

    #[test]
    fn test_if_checksum_does_not_match() {
        // TODO need mocking to avoid outbound calls
    }

    #[test]
    fn test_if_runtime_is_missing() {
        // TODO need mocking to avoid outbound calls
    }
}
