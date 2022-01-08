use std::path::PathBuf;
use std::process::Command;

use libcnb::layer_lifecycle::execute_layer_lifecycle;
use libcnb::{find_one_file, BuildContext, GenericPlatform};

use crate::util::config::{read_package_directories, SFPackageBuildpackConfig};
use crate::{sfdx_delete_org, SFDXLayerLifecycle};

pub fn sfdx(
    context: &BuildContext<GenericPlatform, SFPackageBuildpackConfig>,
) -> Result<Command, anyhow::Error> {
    require_sfdx(context)?;
    Ok(Command::new("sfdx"))
}

pub(crate) fn require_sfdx(
    context: &BuildContext<GenericPlatform, SFPackageBuildpackConfig>,
) -> anyhow::Result<()> {
    if let Ok(output) = Command::new("sfdx").arg("--version").output() {
        let str = String::from_utf8(output.stdout).unwrap();
        assert!(str.contains("sfdx-cli/"));
        return Ok(());
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
    layers_dir: &PathBuf,
    app_dir: &PathBuf,
    hub_user: &str,
    scratch_org_alias: &str,
) -> Result<(), anyhow::Error> {
    println!("---> Resetting environment");
    match sfdx_delete_org(layers_dir, app_dir, hub_user, scratch_org_alias) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
