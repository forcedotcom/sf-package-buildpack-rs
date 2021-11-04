use std::path::PathBuf;
use std::process::Command;

use libcnb::{BuildContext, GenericPlatform};
use libcnb::layer_lifecycle::execute_layer_lifecycle;
use serde::{Deserialize, Serialize};
use toml::value::Table;

pub use crate::layers::sfdx::{sfdx_check_org, sfdx_create_org, sfdx_delete_org, sfdx_push_source, sfdx_test_apex, SFDXLayerLifecycle};
use crate::util::config::read_package_directories;
use crate::util::files::find_one_file;

pub fn sfdx(
    context: &BuildContext<GenericPlatform, SFPackageBuildpackMetadata>,
) -> Result<Command, anyhow::Error> {
    require_sfdx(context)?;
    Ok(Command::new("sfdx"))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SFPackageBuildpackMetadata {
    pub runtime: SFDXRuntime,
}

/// Struct containing the url and sha256 checksum for a downloadable sfdx-runtime-java-runtime.
/// This is used in both `buildpack.toml` and the `layer.toml` but with different keys.
#[derive(Debug, Deserialize, Serialize)]
pub struct SFDXRuntime {
    pub url: String,
    pub manifest: String,
    pub sha256: String,
}

impl SFDXRuntime {
    /// Build a `Runtime` from the `layer.toml`'s `metadata` keys.
    pub fn from_runtime_layer(metadata: &Table) -> Self {
        let empty_string = toml::Value::String("".to_string());
        let url = metadata
            .get("runtime_url")
            .unwrap_or(&empty_string)
            .as_str()
            .unwrap_or("")
            .to_string();
        let manifest = metadata
            .get("runtime_manifest")
            .unwrap_or(&empty_string)
            .as_str()
            .unwrap_or("")
            .to_string();
        let sha256 = metadata
            .get("runtime_sha256")
            .unwrap_or(&empty_string)
            // coerce toml::Value into &str
            .as_str()
            .unwrap_or("")
            .to_string();

        SFDXRuntime {
            url,
            manifest,
            sha256,
        }
    }
}

pub(crate) fn require_sfdx(context: &BuildContext<GenericPlatform, SFPackageBuildpackMetadata>) -> anyhow::Result<()> {
    let use_builtin = std::env::var("CNB_SFDX_USE_BUILTIN");
    if use_builtin.is_err() {
        let output = String::from_utf8(
            Command::new("sfdx")
                .arg("--version")
                .output()
                .expect("failed to execute process")
                .stdout,
        )
            .unwrap();
        if output.contains("sfdx-cli/") {
            return Ok(());
        }
    }
    execute_layer_lifecycle("sfdx", SFDXLayerLifecycle, context)?;
    Ok(())
}

pub(crate) fn find_one_apex_test(app_dir: &PathBuf) -> bool {
    if let Some(vec) = read_package_directories(&app_dir, true, true) {
        for p in vec.iter() {
            if find_one_file(p.as_path(), "IsTest") {
                return true;
            }
        }
    }
    false
}

pub(crate) fn reset_environment(
    app_dir: PathBuf,
    devhub_alias: &str,
    scratch_org_alias: &str,
) {
    println!("---> Resetting environment");
    let output = sfdx_delete_org(&app_dir, devhub_alias, scratch_org_alias).unwrap();
    println!("{:?}", output);
}
