use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Error;
use libcnb::data::layer_content_metadata::LayerContentMetadata;
use libcnb::layer_lifecycle::{LayerLifecycle, ValidateResult};
use libcnb::{BuildContext, GenericPlatform};
use std::env;
use std::io::{BufRead, BufReader};
use std::process::{Command, Output};

use crate::base::SFPackageBuildpackMetadata;
use crate::base::SFDXRuntime;

use crate::util::fetch::{extract, get};

pub struct SFDXLayerLifecycle;

impl
LayerLifecycle<
    GenericPlatform,
    SFPackageBuildpackMetadata,
    SFPackageBuildpackMetadata,
    HashMap<String, String>,
    anyhow::Error,
> for SFDXLayerLifecycle
{
    fn create(
        &self,
        layer_path: &Path,
        build_context: &BuildContext<GenericPlatform, SFPackageBuildpackMetadata>,
    ) -> Result<LayerContentMetadata<SFPackageBuildpackMetadata>, anyhow::Error> {
        let runtime_sha256 = extract(
            &build_context.buildpack_descriptor.metadata.runtime.url,
            &layer_path,
            Some("sfdx/"),
        )?;

        Ok(LayerContentMetadata::default()
            .build(false)
            .cache(true)
            .launch(true)
            .metadata(SFPackageBuildpackMetadata {
                runtime: SFDXRuntime {
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
        layer_content_metadata: &LayerContentMetadata<SFPackageBuildpackMetadata>,
        build_context: &BuildContext<GenericPlatform, SFPackageBuildpackMetadata>,
    ) -> ValidateResult {
        // Fetch most recent sfdx.tar.xz manifest
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
        _layer_content_metadata: LayerContentMetadata<SFPackageBuildpackMetadata>,
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

pub fn sfdx_check_org(
    app_dir: &PathBuf,
    scratch_org_alias: &str,
) -> Result<bool, anyhow::Error> {
    let mut cmd = Command::new("sfdx");
    let result = cmd.current_dir(app_dir)
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
                    devhub_alias, scratch_org_def_path, stderr))
            } else {
                Ok(output)
            }
        }
        Err(e) => {
            Err(anyhow::anyhow!("failed to create scratch org on {} from {} due to {}",
                devhub_alias, scratch_org_def_path, e))
        }
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
                    devhub_alias, scratch_org_alias, stderr));
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
        .expect("failed to execute child");

    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        reader
            .lines()
            .filter_map(|line| line.ok())
            .for_each(|line| eprintln!("{}", line));
    }

    let output = child.wait_with_output().expect("failed to wait on child");
    if output.status.success() {
        Ok(output)
    } else {
        Err(anyhow::anyhow!(
            "failed to push source to {}:\n Exited with {}",
            scratch_org_alias,output.status.code().unwrap()))
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
                return Err(anyhow::anyhow!("failed to run apex tests on {}:\n {}",
                    scratch_org_alias, stderr));
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
