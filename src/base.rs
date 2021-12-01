use std::path::PathBuf;
use std::process::Command;

use libcnb::layer_lifecycle::execute_layer_lifecycle;
use libcnb::{find_one_file, BuildContext, GenericPlatform};

pub use crate::layers::sfdx::{
    sfdx_check_org, sfdx_create_org, sfdx_delete_org, sfdx_push_source, sfdx_test_apex,
    SFDXLayerLifecycle,
};
use crate::util::config::{read_package_directories, SFPackageBuildpackConfig};

pub fn sfdx(
    context: &BuildContext<GenericPlatform, SFPackageBuildpackConfig>,
) -> Result<Command, anyhow::Error> {
    require_sfdx(context)?;
    Ok(Command::new("sfdx"))
}

pub(crate) fn require_sfdx(
    context: &BuildContext<GenericPlatform, SFPackageBuildpackConfig>,
) -> anyhow::Result<()> {
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

pub(crate) fn reset_environment(app_dir: PathBuf, devhub_alias: &str, scratch_org_alias: &str) {
    println!("---> Resetting environment");
    let output = sfdx_delete_org(&app_dir, devhub_alias, scratch_org_alias).unwrap();
    println!("{:?}", output);
}
