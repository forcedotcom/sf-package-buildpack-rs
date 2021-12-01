use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use libcnb::{BuildContext, GenericPlatform, LifecycleMode};
use libcnb::Error::BuildpackError;

pub use crate::layers::sfdx::{sfdx_check_org, sfdx_create_org, sfdx_delete_org, sfdx_push_source, sfdx_test_apex, SFDXLayerLifecycle};

use crate::layers::sfdx::{sfdx_create_package, sfdx_create_package_version, sfdx_find_package};
use crate::{find_one_apex_test, require_sfdx, reset_environment};
use crate::util::config::{SFPackageAppConfig, SFPackageBuildpackConfig};
use crate::util::logger::{BuildLogger, Logger};
use crate::util::meta::{write_package_meta, write_package_version_meta};

pub fn build(context: BuildContext<GenericPlatform, SFPackageBuildpackConfig>) -> libcnb::Result<(), anyhow::Error> {
    let mut logger = BuildLogger::new(true);

    require_sfdx(&context)?;

    let lifecycle_mode = env::var("CNB_LIFECYCLE_MODE").unwrap_or(String::from("CI"));
    let mode = LifecycleMode::from_str(lifecycle_mode.as_str())?;

    // Lifecycle Mode => Dev, CI, Test, or Package
    // Dev => namespaced scratch org created if needed, source push, test run if desired, setup automation if desired.  Use file watcher to trigger if desired.
    // CI => namespaced scratch org created, source push, test run, scratch org deleted
    // Test (Install) => beta package version built, non-namespaced extended scratch org created, dependent packages installed, beta package version installed, setup automation if desired
    // Test (Upgrade) => beta package version built, non-namespaced extended scratch org created, dependent packages installed, ancestor released package version installed, setup automation if desired, beta package version installed
    // Package => beta package version promoted, published
    match mode {
        LifecycleMode::Dev => dev_build(context, &mut logger).map_err(BuildpackError),
        LifecycleMode::CI => ci_build(context, &mut logger).map_err(BuildpackError),
        LifecycleMode::Package => package_build(context, &mut logger).map_err(BuildpackError),
        _ => Ok(()),
    }
}

fn dev_build(context: BuildContext<GenericPlatform, SFPackageBuildpackConfig>, logger: &mut BuildLogger) -> Result<(), anyhow::Error> {
    let app_dir = context.app_dir;

    // TODO make configurable
    let hub_alias = "hub";
    let org_alias = "dev";
    let org_def_path = "config/project-scratch-def.json";
    let org_duration = 15;
    let op_wait_seconds = 120;
    let run_tests = false;

    logger.header("---> Creating environment")?;
    if !sfdx_check_org(&app_dir, org_alias)? {
        logger.info("---> Creating scratch org")?;
        let output = sfdx_create_org(&app_dir, hub_alias, org_def_path, org_duration, org_alias)?;
        logger.output("creating environment", output)?;
    }

    logger.header("---> Preparing artifacts")?;

    let proceed = push_source(logger, &app_dir, org_alias, op_wait_seconds)?;

    if proceed && run_tests {
        logger.header("---> Running tests")?;

        if find_one_apex_test(&app_dir) {
            logger.info("---> Running apex tests")?;
            match sfdx_test_apex(&app_dir, org_alias, app_dir.join("results"), 240) {
                Ok(output) => {
                    logger.output("running tests", output)?;
                }
                Err(e) => {
                    logger.error("running tests", e)?;
                }
            }
        }
    }

    Ok(())
}

fn push_source(logger: &mut BuildLogger, app_dir: &PathBuf, org_alias: &str, dev_op_wait_seconds: i32) -> Result<bool, anyhow::Error> {
    logger.info("---> Pushing source code")?;
    let mut succeeded = true;
    match sfdx_push_source(&app_dir, org_alias, dev_op_wait_seconds) {
        Ok(output) => {
            logger.output("preparing artifacts", output)?;
        },
        Err(e) => {
            logger.error("preparing artifacts", e)?;
            succeeded = false;
        }
    }
    Ok(succeeded)
}

fn ci_build(context: BuildContext<GenericPlatform, SFPackageBuildpackConfig>, logger: &mut BuildLogger) -> Result<(), anyhow::Error> {
    let app_dir = context.app_dir;

    // TODO make configurable
    let hub_alias = "hub";
    let org_alias = "ci";
    let org_def_path = "config/project-scratch-def.json";
    let org_duration_days = 1;
    let op_wait_seconds = 120;

    logger.header("---> Creating environment")?;

    logger.info("---> Creating scratch org")?;
    let output = sfdx_create_org(&app_dir, hub_alias, org_def_path, org_duration_days, org_alias)?;
    logger.output("creating environment", output)?;

    logger.header("---> Preparing artifacts")?;

    logger.info("---> Pushing source code")?;
    let mut abort = false;
    match sfdx_push_source(&app_dir, org_alias, op_wait_seconds) {
        Ok(output) => {
            logger.output("preparing artifacts", output)?;
        },
        Err(e) => {
            logger.error("preparing artifacts", e)?;
            abort = true;
        }
    }

    if !abort {
        logger.header("---> Running tests")?;

        if find_one_apex_test(&app_dir) {
            logger.info("---> Running apex tests")?;
            match sfdx_test_apex(&app_dir, org_alias, app_dir.join("results"), 240) {
                Ok(output) => {
                    logger.output("running tests", output)?;
                }
                Err(e) => {
                    logger.error("running tests", e)?;
                }
            }
        }
    }

    reset_environment(app_dir, hub_alias, org_alias);
    Ok(())
}

fn package_build(context: BuildContext<GenericPlatform, SFPackageBuildpackConfig>, logger: &mut BuildLogger) -> Result<(), anyhow::Error> {
    let app_dir = context.app_dir;

    let app_config = SFPackageAppConfig::from_dir(&app_dir);
    let package_config = app_config.package;

    logger.header("---> Preparing artifacts")?;
    let mut package_id= package_config.id;
    if package_id.is_empty() && package_config.create_if_needed {
        let found_response = sfdx_find_package(&app_dir, &package_config.hub_alias, &package_config.name)?;
        if found_response.result.package_id.is_empty() {
            logger.info("---> Creating package")?;
            let response = sfdx_create_package(&app_dir, &package_config.hub_alias, &package_config.name, &package_config.description, &package_config.package_type, &package_config.root)?;
            package_id = response.result.package_id;
        } else {
            package_id = found_response.result.package_id;
        }
        write_package_meta(&app_dir, &package_id, &package_config.name, &package_config.hub_alias)?;
    }

    logger.info("---> Building package version")?;
    match sfdx_create_package_version(&app_dir, &package_config.hub_alias, &package_id,
                                      &package_config.org_def_path, &package_config.version_name,
                                      &package_config.version_number,
                                      &package_config.installation_key,
                                      package_config.op_wait_seconds) {
        Ok(v) => {
            let result = &v["result"];
            write_package_version_meta(&app_dir, &package_config.version_name, &package_config.version_number, &package_id,
                                       result["SubscriberPackageVersionId"].as_str().unwrap().to_string(),
                                       result["CreatedDate"].as_str().unwrap().to_string())?;
            logger.info("New package version created")?;
        },
        Err(e) => {
            logger.error("preparing artifacts", e)?;
        }
    }
    Ok(())
}
