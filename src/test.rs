use crate::util::config::{SFPackageAppConfig, SFPackageBuildpackConfig};
use crate::{push_source, reset_environment, sfdx_create_org, sfdx_test_apex, BuildLogger, Logger};
use anyhow::anyhow;
use libcnb::Error::BuildpackError;
use libcnb::{
    get_lifecycle_mode, GenericPlatform, LifecycleMode, TestContext, TestOutcome, TestResults,
};

/// # Execute Tests Command
/// A full test command differs from unit tests run during the build. Test should involve more
/// substantial automations, running longer and more in depth validations, code coverage, etc.
pub fn test(
    context: TestContext<GenericPlatform, SFPackageBuildpackConfig>,
) -> libcnb::Result<TestOutcome, anyhow::Error> {
    let mut logger = BuildLogger::new(true, true);

    let mode = get_lifecycle_mode().unwrap_or(LifecycleMode::Dev);
    match mode {
        LifecycleMode::Dev => dev_test(context, &mut logger),
        LifecycleMode::CI => ci_test(context, &mut logger),
        LifecycleMode::Package => package_test(context, &mut logger),
        _ => Ok(TestOutcome::Pass(TestResults::new())),
    }
}

/// # Dev Mode Test
/// Execute tests in an existing scratch org, formatted for interactive developer consumption.
fn dev_test(
    context: TestContext<GenericPlatform, SFPackageBuildpackConfig>,
    logger: &mut BuildLogger,
) -> libcnb::Result<TestOutcome, anyhow::Error> {
    let config = SFPackageAppConfig::from_dir(&context.app_dir).dev;
    match sfdx_test_apex(
        &context.layers_dir,
        &context.app_dir,
        &config.org_alias,
        config.test_results_path,
        config.test_results_format,
        config.op_wait_seconds,
    ) {
        Ok(result) => {
            let outcome = result.into();
            match &outcome {
                TestOutcome::Pass(results) => {
                    logger.info("Test run succeeded")?;
                    logger.info(format!("{} tests passed", results.passed.len()))?;
                    logger.info(format!("{} tests failed", results.failed.len()))?;
                    logger.info(format!("{} tests ignored", results.ignored.len()))?;
                }
                TestOutcome::Fail(results) => {
                    logger.info("Test run completed with failures")?;
                    logger.info(format!("{} tests passed", results.passed.len()))?;
                    logger.info(format!("{} tests failed", results.failed.len()))?;
                    logger.info(format!("{} tests ignored", results.ignored.len()))?;
                }
            }
            Ok(outcome)
        }
        Err(e) => libcnb::Result::Err(BuildpackError(e)),
    }
}

/// # CI Mode Test
/// Execute tests for a CI container, creating and cleaning up scratch org.
fn ci_test(
    context: TestContext<GenericPlatform, SFPackageBuildpackConfig>,
    logger: &mut BuildLogger,
) -> libcnb::Result<TestOutcome, anyhow::Error> {
    let app_dir = &context.app_dir;
    let layers_dir = &context.layers_dir;
    let config = SFPackageAppConfig::from_dir(app_dir).ci;

    sfdx_create_org(
        layers_dir,
        app_dir,
        &config.hub_user,
        &config.org_def_path,
        config.org_duration_days,
        &config.org_alias,
    )?;

    logger.header("---> Preparing artifacts")?;

    let result = match push_source(
        layers_dir,
        logger,
        app_dir,
        &config.org_alias,
        config.op_wait_seconds,
    )? {
        true => {
            match sfdx_test_apex(
                layers_dir,
                app_dir,
                &config.org_alias,
                config.test_results_path,
                config.test_results_format,
                config.op_wait_seconds,
            ) {
                Ok(result) => {
                    let outcome = result.into();
                    match &outcome {
                        TestOutcome::Pass(results) => {
                            logger.info("Test run succeeded")?;
                            logger.info(format!("{} tests passed", results.passed.len()))?;
                            logger.info(format!("{} tests failed", results.failed.len()))?;
                            logger.info(format!("{} tests ignored", results.ignored.len()))?;
                        }
                        TestOutcome::Fail(results) => {
                            logger.info("Test run completed with failures")?;
                            logger.info(format!("{} tests passed", results.passed.len()))?;
                            logger.info(format!("{} tests failed", results.failed.len()))?;
                            logger.info(format!("{} tests ignored", results.ignored.len()))?;
                        }
                    }
                    Ok(outcome)
                }
                Err(e) => libcnb::Result::Err(BuildpackError(e)),
            }
        }
        false => Err(BuildpackError(anyhow!(
            "No tests were executed.  Failed to push source."
        ))),
    };

    reset_environment(layers_dir, app_dir, &config.hub_user, &config.org_alias)?;
    result
}

/// # Package Mode Test
/// TODO Should involve installation of a built package artifact and suitable tests to verify it.
fn package_test(
    _context: TestContext<GenericPlatform, SFPackageBuildpackConfig>,
    _logger: &mut BuildLogger,
) -> libcnb::Result<TestOutcome, anyhow::Error> {
    Ok(TestOutcome::Pass(TestResults::new()))
}
