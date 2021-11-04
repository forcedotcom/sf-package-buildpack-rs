use std::env;
use std::path::PathBuf;

use libcnb::{BuildContext, GenericPlatform, LifecycleMode};
use libcnb::Error::BuildpackError;

pub use crate::layers::sfdx::{sfdx_check_org, sfdx_create_org, sfdx_delete_org, sfdx_push_source, sfdx_test_apex, SFDXLayerLifecycle};
use crate::{base, SFPackageBuildpackMetadata};
use crate::util::logger::{BuildLogger, Logger};

pub fn build(context: BuildContext<GenericPlatform, SFPackageBuildpackMetadata>) -> libcnb::Result<(), anyhow::Error> {
    let mut logger = BuildLogger::new(true);

    base::require_sfdx(&context)?;

    let lifecycle_mode = env::var("CNB_LIFECYCLE_MODE").unwrap_or(String::from("CI"));
    let mode = LifecycleMode::from(lifecycle_mode);

    // Lifecycle Mode => Dev, CI, Test, or Prod
    // Dev => namespaced scratch org created if needed, source push, test run if desired, setup automation if desired.  Use file watcher to trigger if desired.
    // CI => namespaced scratch org created, source push, test run, scratch org deleted
    // Test (Install) => beta package version built, non-namespaced extended scratch org created, dependent packages installed, beta package version installed, setup automation if desired
    // Test (Upgrade) => beta package version built, non-namespaced extended scratch org created, dependent packages installed, ancestor released package version installed, setup automation if desired, beta package version installed
    // Package => beta package version promoted, published
    match mode {
        LifecycleMode::Dev => dev_build(context, &mut logger).map_err(BuildpackError),
        LifecycleMode::CI => ci_build(context, &mut logger).map_err(BuildpackError),
        _ => Ok(()),
    }
}

fn dev_build(context: BuildContext<GenericPlatform, SFPackageBuildpackMetadata>, logger: &mut BuildLogger) -> Result<(), anyhow::Error> {
    let app_dir = context.app_dir;

    // TODO make configurable
    let devhub_alias = "hub";
    let scratch_org_alias = "dev";
    let scratch_org_def_path = "config/project-scratch-def.json";
    let scratch_org_duration = 15;
    let dev_op_wait_seconds = 120;
    let dev_run_tests = false;

    logger.header("---> Creating environment")?;
    if !sfdx_check_org(&app_dir, scratch_org_alias)? {
        logger.info("---> Creating scratch org")?;
        let output = sfdx_create_org(&app_dir, devhub_alias, scratch_org_def_path, scratch_org_duration, scratch_org_alias)?;
        logger.output("creating environment", output)?;
    }

    logger.header("---> Preparing artifacts")?;

    let proceed = push_source(logger, &app_dir, scratch_org_alias, dev_op_wait_seconds)?;

    if proceed && dev_run_tests {
        logger.header("---> Running tests")?;

        if base::find_one_apex_test(&app_dir) {
            logger.info("---> Running apex tests")?;
            match sfdx_test_apex(&app_dir, scratch_org_alias, app_dir.join("results"), 240) {
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

fn push_source(logger: &mut BuildLogger, app_dir: &PathBuf, scratch_org_alias: &str, dev_op_wait_seconds: i32) -> Result<bool, anyhow::Error> {
    logger.info("---> Pushing source code")?;
    let mut succeeded = true;
    match sfdx_push_source(&app_dir, scratch_org_alias, dev_op_wait_seconds) {
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

fn ci_build(context: BuildContext<GenericPlatform, SFPackageBuildpackMetadata>, logger: &mut BuildLogger) -> Result<(), anyhow::Error> {
    let app_dir = context.app_dir;

    // TODO make configurable
    let devhub_alias = "hub";
    let scratch_org_alias = "ci";
    let scratch_org_def_path = "config/project-scratch-def.json";
    let scratch_org_duration_days = 1;
    let ci_op_wait_seconds = 120;

    logger.header("---> Creating environment")?;

    logger.info("---> Creating scratch org")?;
    let output = sfdx_create_org(&app_dir, devhub_alias, scratch_org_def_path, scratch_org_duration_days, scratch_org_alias)?;
    logger.output("creating environment", output)?;

    logger.header("---> Preparing artifacts")?;

    logger.info("---> Pushing source code")?;
    let mut abort = false;
    match sfdx_push_source(&app_dir, scratch_org_alias, ci_op_wait_seconds) {
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

        if base::find_one_apex_test(&app_dir) {
            logger.info("---> Running apex tests")?;
            match sfdx_test_apex(&app_dir, scratch_org_alias, app_dir.join("results"), 240) {
                Ok(output) => {
                    logger.output("running tests", output)?;
                }
                Err(e) => {
                    logger.error("running tests", e)?;
                }
            }
        }
    }

    base::reset_environment(app_dir, devhub_alias, scratch_org_alias);
    Ok(())
}
