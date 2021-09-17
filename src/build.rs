use crate::data::buildpack_toml::SFPackageBuildpackMetadata;
use crate::layers::sfdx::SFDXLayerLifecycle;
use libcnb::{layer_lifecycle::execute_layer_lifecycle, BuildContext, GenericPlatform};
use std::fmt::Debug;
use std::path::PathBuf;
use std::process::{Command, Output};

enum LifecycleMode {
    Dev,
    CI,
    Test,
    Prod,
}

impl LifecycleMode {
    pub fn from(os_str: String) -> Self {
        match os_str.as_str() {
            "Dev" => LifecycleMode::Dev,
            "Test" => LifecycleMode::Test,
            "Prod" => LifecycleMode::Prod,
            "CI" | _ => LifecycleMode::CI,
        }
    }
}

/// `bin/build`
pub fn build(
    context: BuildContext<GenericPlatform, SFPackageBuildpackMetadata>,
) -> Result<(), libcnb::Error<anyhow::Error>> {
    let mut logger = BuildLogger::new(true);

    log_info(&mut logger, "---> Initializing buildpack");
    require_sfdx(&context)?;

    let lifecycle_mode = env::var("CNB_LIFECYCLE_MODE")
        .unwrap_or(String::from("CI"));
    let mode = LifecycleMode::from(lifecycle_mode);

    // Lifecycle Mode => Dev, CI, Test, or Prod
    // Dev => namespaced scratch org created if needed, source push, test run if desired, setup automation if desired.  Use file watcher to trigger if desired.
    // CI => namespaced scratch org created, source push, test run, scratch org deleted
    // Test (Install) => beta package version built, non-namespaced extended scratch org created, dependent packages installed, beta package version installed, setup automation if desired
    // Test (Upgrade) => beta package version built, non-namespaced extended scratch org created, dependent packages installed, ancestor released package version installed, setup automation if desired, beta package version installed
    // Prod => beta package version promoted, published
    match mode {
        LifecycleMode::CI => {
            let app_dir = context.app_dir;

            // TODO make configurable
            let devhub_alias = "hub";
            let scratch_org_alias = "ci";
            let scratch_org_def_path = "config/project-scratch-def.json";
            let scratch_org_duration = 1;

            log_header(&mut logger, "---> Creating environment");
            match sfdx_create_org(
                app_dir.clone(),
                devhub_alias,
                scratch_org_def_path,
                scratch_org_duration,
                scratch_org_alias,
            ) {
                Ok(output) => {
                    log_output(&mut logger, "creating environment", output);
                }
                Err(e) => {
                    return Err(libcnb::Error::BuildpackError(anyhow::Error::from(e)));
                }
            }

            log_header(&mut logger, "---> Preparing artifacts");
            match sfdx_push_source(app_dir.clone(), scratch_org_alias, 120) {
                Ok(output) => log_output(&mut logger, "preparing artifacts", output),
                Err(e) => {
                    reset_environment(&mut logger, app_dir, devhub_alias, scratch_org_alias);
                    return Err(libcnb::Error::BuildpackError(anyhow::Error::from(e)));
                }
            }

            if find_one_apex_test(&app_dir) {
                log_header(&mut logger, "---> Running tests");
                match sfdx_test_apex(
                    app_dir.clone(),
                    scratch_org_alias,
                    app_dir.join("results"),
                    240,
                ) {
                    Ok(output) => {
                        log_output(&mut logger, "running tests", output);
                    }
                    Err(e) => {
                        reset_environment(&mut logger, app_dir, devhub_alias, scratch_org_alias);
                        return Err(libcnb::Error::BuildpackError(anyhow::Error::from(e)));
                    }
                }
            }

            reset_environment(&mut logger, app_dir, devhub_alias, scratch_org_alias);

            Ok(())
        }
        _ => Ok(()),
    }
}

pub fn sfdx(context: &BuildContext<GenericPlatform, SFPackageBuildpackMetadata>) -> Result<Command, libcnb::Error<anyhow::Error>> {
    require_sfdx(context)?;
    Ok(Command::new("sfdx"))
}

fn require_sfdx(context: &BuildContext<GenericPlatform, SFPackageBuildpackMetadata>) -> Result<bool, libcnb::Error<anyhow::Error>> {
    let use_builtin = env::var("CNB_SFDX_USE_BUILTIN");
    if use_builtin.is_err() {
        let output = String::from_utf8(Command::new("sfdx")
            .arg("--version")
            .output()
            .expect("failed to execute process").stdout).unwrap();
        if output.contains("sfdx-cli/") {
            return Ok(false);
        }
    }
    execute_layer_lifecycle("sfdx", SFDXLayerLifecycle, &context)?;
    Ok(true)
}

fn find_one_apex_test(app_dir: &PathBuf) -> bool {
    if let Some(vec) = read_package_directories(&app_dir, true, true) {
        for p in vec.iter() {
            if find_one_file(p.as_path(), "IsTest") {
                return true;
            }
        }
    }
    false
}

fn reset_environment(
    mut logger: &mut BuildLogger,
    app_dir: PathBuf,
    devhub_alias: &str,
    scratch_org_alias: &str,
) {
    log_header(&mut logger, "---> Resetting environment");
    let output = sfdx_delete_org(app_dir, devhub_alias, scratch_org_alias).unwrap();
    log_output(&mut logger, "resetting environment", output);
}

fn log_header(logger: &mut BuildLogger, header: &str) {
    logger.header(header).unwrap();
}

fn log_info(logger: &mut BuildLogger, info: &str) {
    logger.info(info).unwrap();
}

fn log_output(logger: &mut BuildLogger, header: &str, output: Output) {
    let status = output.status;
    if !&output.stdout.is_empty() {
        logger
            .debug(format!("---> {}", String::from_utf8_lossy(&output.stdout)))
            .unwrap();
    }
    if !&output.stderr.is_empty() {
        if status.success() {
            // Yes, some sfdx commands like force:source:push decided to output progress to stderr.
            logger
                .info(format!("---> {}", String::from_utf8_lossy(&output.stderr)))
                .unwrap();
        } else {
            logger
                .error(
                    format!("---> Failed {}", header),
                    format!("---> {}", String::from_utf8_lossy(&output.stderr)),
                )
                .unwrap();
        }
    }
}

#[derive(Debug)]
struct BuildError(String);

use crate::util::config::read_package_directories;
use crate::util::files::find_one_file;
use crate::util::logger::{BuildLogger, Logger};
use std::{error, env};
use std::fmt;

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "There is an error: {}", self.0)
    }
}

impl error::Error for BuildError {}

fn sfdx_create_org(
    app_dir: PathBuf,
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
                return Err(anyhow::Error::new(BuildError(format!(
                    "failed to create scratch org on {} from {}:\n{}",
                    devhub_alias, scratch_org_def_path, stderr
                ))));
            }
            Ok(output)
        }
        Err(e) => {
            println!(
                "failed to create scratch org on {} from {}",
                devhub_alias, scratch_org_def_path
            );
            Err(anyhow::Error::new(e))
        }
    }
}

fn sfdx_delete_org(
    app_dir: PathBuf,
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
                return Err(anyhow::Error::new(BuildError(format!(
                    "failed to delete scratch org on {} named {}:\n {}",
                    devhub_alias, scratch_org_alias, stderr
                ))));
            }
            Ok(output)
        }
        Err(e) => {
            println!(
                "failed to delete scratch org on {} named {}",
                devhub_alias, scratch_org_alias
            );
            Err(anyhow::Error::new(e))
        }
    }
}

fn sfdx_push_source(
    app_dir: PathBuf,
    scratch_org_alias: &str,
    wait_seconds: i32,
) -> Result<Output, anyhow::Error> {
    let mut cmd = Command::new("sfdx");
    cmd.current_dir(app_dir)
        .arg("force:source:push")
        .arg("-u")
        .arg(scratch_org_alias)
        .arg("-w")
        .arg(wait_seconds.to_string());
    match cmd.output() {
        Ok(output) => {
            let status = output.status.code().unwrap();
            let stderr = String::from_utf8(output.stderr.to_owned()).unwrap();
            if status != 0 {
                return Err(anyhow::Error::new(BuildError(format!(
                    "failed to push source to {}:\n {}",
                    scratch_org_alias, stderr
                ))));
            }
            Ok(output)
        }
        Err(e) => {
            print!("failed to push source to {}", scratch_org_alias);
            Err(anyhow::Error::new(e))
        }
    }
}

fn sfdx_test_apex(
    app_dir: PathBuf,
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
                return Err(anyhow::Error::new(BuildError(format!(
                    "failed to run apex tests on {}:\n {}",
                    scratch_org_alias, stderr
                ))));
            }
            Ok(output)
        }
        Err(e) => {
            println!("failed to run apex tests on {}", scratch_org_alias);
            Err(anyhow::Error::new(e))
        }
    }
}
