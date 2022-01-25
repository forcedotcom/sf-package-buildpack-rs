use std::path::PathBuf;

use libcnb::Error::BuildpackError;
use libcnb::{get_lifecycle_mode, BuildContext, GenericPlatform, LifecycleMode};

use crate::layers::sfdx::{
    sfdx_auth, sfdx_create_org, sfdx_create_package, sfdx_create_package_version,
    sfdx_find_package, sfdx_push_source, sfdx_test_apex,
};
use crate::util::config::{SFPackageAppConfig, SFPackageBuildpackConfig};
use crate::util::logger::{BuildLogger, Logger};
use crate::util::meta::{write_package_meta, write_package_version_meta};
use crate::{find_one_apex_test, require_sfdx, reset_environment, sfdx_create_org_if_needed};

pub fn build(
    context: BuildContext<GenericPlatform, SFPackageBuildpackConfig>,
) -> libcnb::Result<(), anyhow::Error> {
    let mut logger = BuildLogger::new(true, true);

    require_sfdx(&context)?;

    let mode = get_lifecycle_mode().unwrap_or(LifecycleMode::Dev);

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

pub fn dev_build(
    context: BuildContext<GenericPlatform, SFPackageBuildpackConfig>,
    logger: &mut BuildLogger,
) -> Result<(), anyhow::Error> {
    let app_dir = &context.app_dir;
    let layers_dir = &context.layers_dir;

    let config = SFPackageAppConfig::from_dir(app_dir).dev;

    logger.header("---> Dev Build")?;

    logger.header("---> Creating environment")?;

    sfdx_auth(
        layers_dir,
        &context.app_dir,
        &config.hub_client_id,
        &config.hub_key_path,
        &config.hub_instance_url,
        &config.hub_user,
        config.hub_alias,
    )?;

    sfdx_create_org_if_needed(
        layers_dir,
        app_dir,
        &config.hub_user,
        &config.org_def_path,
        config.org_duration_days,
        &config.org_alias,
        logger,
    )?;

    logger.header("---> Preparing artifacts")?;

    let proceed = push_source(
        layers_dir,
        logger,
        app_dir,
        &config.org_alias,
        config.op_wait_seconds,
    )?;

    if proceed && config.run_tests {
        logger.header("---> Running tests")?;

        if find_one_apex_test(app_dir) {
            logger.info("---> Running apex tests")?;
            match sfdx_test_apex(
                layers_dir,
                app_dir,
                &config.org_alias,
                config.test_results_path,
                config.test_results_format,
                240,
            ) {
                Ok(result) => {
                    logger.info(format!("{:?}", result))?;
                }
                Err(e) => {
                    logger.error("running tests", e)?;
                }
            }
        }
    }

    Ok(())
}

pub fn push_source(
    layers_dir: &PathBuf,
    logger: &mut BuildLogger,
    app_dir: &PathBuf,
    org_alias: &str,
    dev_op_wait_seconds: i32,
) -> Result<bool, anyhow::Error> {
    logger.info("---> Pushing source code")?;
    let mut succeeded = true;
    match sfdx_push_source(layers_dir, app_dir, org_alias, dev_op_wait_seconds) {
        Ok(output) => {
            logger.output("preparing artifacts", output)?;
        }
        Err(e) => {
            logger.error("preparing artifacts", e)?;
            succeeded = false;
        }
    }
    Ok(succeeded)
}

pub fn ci_build(
    context: BuildContext<GenericPlatform, SFPackageBuildpackConfig>,
    logger: &mut BuildLogger,
) -> Result<(), anyhow::Error> {
    logger.header("---> CI Build")?;
    let app_dir = &context.app_dir;
    let config = SFPackageAppConfig::from_dir(app_dir).ci;

    logger.header("---> Creating environment")?;
    sfdx_auth(
        &context.layers_dir,
        &context.app_dir,
        &config.hub_client_id,
        &config.hub_key_path,
        &config.hub_instance_url,
        &config.hub_user,
        config.hub_alias,
    )?;

    logger.info("---> Creating scratch org")?;
    let output = sfdx_create_org(
        &context.layers_dir,
        app_dir,
        &config.hub_user,
        &config.org_def_path,
        config.org_duration_days,
        &config.org_alias,
    )?;
    logger.output("creating environment", output)?;

    logger.header("---> Preparing artifacts")?;

    logger.info("---> Pushing source code")?;
    let mut abort = false;
    match sfdx_push_source(
        &context.layers_dir,
        app_dir,
        &config.org_alias,
        config.op_wait_seconds,
    ) {
        Ok(output) => {
            logger.output("preparing artifacts", output)?;
        }
        Err(e) => {
            logger.error("preparing artifacts", e)?;
            abort = true;
        }
    }

    if !abort {
        logger.header("---> Running tests")?;

        if find_one_apex_test(app_dir) {
            logger.info("---> Running apex tests")?;
            match sfdx_test_apex(
                &context.layers_dir,
                app_dir,
                &config.org_alias,
                config.test_results_path,
                config.test_results_format,
                240,
            ) {
                Ok(result) => {
                    logger.info(format!("{:?}", result))?;
                }
                Err(e) => {
                    logger.error("running tests", e)?;
                }
            }
        }
    }

    reset_environment(
        &context.layers_dir,
        app_dir,
        &config.hub_user,
        &config.org_alias,
    )?;
    Ok(())
}

pub fn package_build(
    context: BuildContext<GenericPlatform, SFPackageBuildpackConfig>,
    logger: &mut BuildLogger,
) -> Result<(), anyhow::Error> {
    let layers_dir = &context.layers_dir;
    let app_dir = &context.app_dir;

    logger.header("---> Package Build")?;

    let config = SFPackageAppConfig::from_dir(app_dir).package;

    sfdx_auth(
        &context.layers_dir,
        &context.app_dir,
        &config.hub_client_id,
        &config.hub_key_path,
        &config.hub_instance_url,
        &config.hub_user,
        config.hub_alias,
    )?;

    logger.header("---> Preparing artifacts")?;
    let mut package_id = config.id;
    if package_id.is_empty() && config.create_if_needed {
        let found_response =
            sfdx_find_package(layers_dir, app_dir, &config.hub_user, &config.name)?;
        if found_response.result.package_id.is_empty() {
            logger.info("---> Creating package")?;
            let response = sfdx_create_package(
                layers_dir,
                app_dir,
                &config.hub_user,
                &config.name,
                &config.description,
                &config.package_type,
                &config.root,
            )?;
            package_id = response.result.package_id;
        } else {
            package_id = found_response.result.package_id;
        }
        write_package_meta(
            app_dir,
            &package_id,
            &config.name,
            &config.hub_user,
            &config.hub_instance_url,
        )?;
    }

    logger.info("---> Building package version")?;
    match sfdx_create_package_version(
        layers_dir,
        app_dir,
        &config.hub_user,
        &package_id,
        &config.org_def_path,
        &config.version_name,
        &config.version_number,
        &config.installation_key,
        config.op_wait_seconds,
    ) {
        Ok(result) => {
            write_package_version_meta(
                app_dir,
                result.subscriber_package_version_id,
                package_id,
                config.version_name,
                result.version,
            )?;
            logger.info("New package version created")?;
        }
        Err(e) => {
            logger.error("preparing artifacts", e)?;
        }
    }
    Ok(())
}
