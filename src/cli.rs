use anyhow::{anyhow, Error};
use std::path::{Path, PathBuf};
use std::{env, process};

use crate::util::enc_file;
use crate::util::enc_file::EncFile;
use crate::{BuildLogger, Logger};
use clap::{App, AppSettings, Arg, ArgMatches, ArgSettings};
use libcnb::data::buildpack_plan::{BuildpackPlan, Entry};
use libcnb::{
    read_file_to_string, set_lifecycle_mode, BuildContext, DetectContext, DetectOutcome,
    GenericPlatform, Platform, PublishContext, TestContext, TestOutcome,
};

pub fn cli() {
    if self::execute(env::args().collect()).is_err() {
        process::exit(1);
    }
}

pub fn execute(args: Vec<String>) -> Result<(), anyhow::Error> {
    let mut logger = BuildLogger::new(true, false);

    let app = App::new("SF Package Buildpack CLI")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            App::new("pack")
                .about("Buildpack commands")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .arg(
                    Arg::new("mode")
                        .help("Override the CNB_LIFECYCLE_MODE variable with a particular mode for this command")
                        .takes_value(true)
                        .long("mode")
                        .short('m')
                )
                .subcommand(App::new("detect")
                    .about("Detect whether this buildpack can build the application")
                    .setting(AppSettings::ArgRequiredElseHelp)
                    .arg(
                        Arg::new("source")
                            .help("path to the application source directory, containing the app.toml file")
                            .setting(ArgSettings::Required)
                    )
                    .arg(
                        Arg::new("platform")
                            .help("path to a directory containing platform provided configuration, for cloud native buildpacks.  Files containing environment variables should reside within an env subdirectory here.")
                            .takes_value(true)
                            .long("platform")
                            .short('p')
                    ).arg(
                        Arg::new("env")
                            .help("path to a directory with files containing environment variable values")
                            .takes_value(true)
                            .long("env")
                            .short('e')
                    )
                    .arg(
                    Arg::new("layers")
                        .help("path to a directory able to cache dependencies")
                        .takes_value(true)
                        .long("layers")
                        .short('l')
                )
                )
                .subcommand(
                    App::new("build")
                        .about("build the application")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(
                            Arg::new("source")
                                .help("path to the application source directory, containing the app.toml file")
                                .setting(ArgSettings::Required)
                        )
                        .arg(
                            Arg::new("platform")
                                .help("path to a directory containing platform provided configuration, for cloud native buildpacks.  Files containing environment variables should reside within an env subdirectory here.")
                                .takes_value(true)
                                .long("platform")
                                .short('p')
                        ).arg(
                            Arg::new("env")
                                .help("path to a directory with files containing environment variable values")
                                .takes_value(true)
                                .long("env")
                                .short('e')
                        )
                        .arg(
                            Arg::new("layers")
                                .help("path to a directory able to cache dependencies")
                                .takes_value(true)
                                .long("layers")
                                .short('l')
                        )
                )
                .subcommand(
                    App::new("test")
                        .about("test the application")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(
                            Arg::new("source")
                                .help("path to the application source directory, containing the app.toml file")
                                .setting(ArgSettings::Required)
                        )
                        .arg(
                            Arg::new("platform")
                                .help("path to a directory containing platform provided configuration, for cloud native buildpacks.  Files containing environment variables should reside within an env subdirectory here.")
                                .takes_value(true)
                                .long("platform")
                                .short('p')
                        ).arg(
                            Arg::new("env")
                                .help("path to a directory with files containing environment variable values")
                                .takes_value(true)
                                .long("env")
                                .short('e')
                        )
                        .arg(
                            Arg::new("layers")
                                .help("path to a directory able to cache dependencies")
                                .takes_value(true)
                                .long("layers")
                                .short('l')
                        )
                )
                .subcommand(App::new("publish")
                    .about("publish the application")
                    .setting(AppSettings::ArgRequiredElseHelp)
                    .arg(
                        Arg::new("source")
                            .help("path to the application source directory, containing the app.toml file")
                            .setting(ArgSettings::Required)
                    )
                    .arg(
                        Arg::new("platform")
                            .help("path to a directory containing platform provided configuration, for cloud native buildpacks.  Files containing environment variables should reside within an env subdirectory here.")
                            .takes_value(true)
                            .long("platform")
                            .short('p')
                    ).arg(
                        Arg::new("env")
                            .help("path to a directory with files containing environment variable values")
                            .takes_value(true)
                            .long("env")
                            .short('e')
                    )
                    .arg(
                        Arg::new("layers")
                            .help("path to a directory able to cache dependencies")
                            .takes_value(true)
                            .long("layers")
                            .short('l')
                    )
                ),
        )
        .subcommand(
            App::new("file")
                .about("File-related utility commands")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    App::new("encrypt")
                        .about("Encrypt a file using openssl ciphers")
                        .arg(
                            Arg::new("source")
                                .help("The path to the file to be encrypted")
                                .setting(ArgSettings::Required)
                        )
                        .arg(
                            Arg::new("target")
                                .help("The path to the new encrypted file")
                                .setting(ArgSettings::Required)
                        )
                        .arg(
                            Arg::new("ssl_key")
                                .help("The encryption key to use")
                                .short('k')
                                .takes_value(true)
                        )
                        .arg(
                            Arg::new("ssl_iv")
                                .help("The encryption initialization vector to use")
                                .short('v')
                                .takes_value(true)
                        ),
                )
                .subcommand(
                    App::new("decrypt")
                        .about("Decrypt a file using openssl ciphers")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .arg(
                            Arg::new("source")
                                .help("The path to the file to be decrypted")
                                .setting(ArgSettings::Required)
                        )
                        .arg(
                            Arg::new("target")
                                .help("The path to the new decrypted file")
                                .setting(ArgSettings::Required)
                        )
                        .arg(
                            Arg::new("ssl_key")
                                .help("The encryption key to use")
                                .short('k')
                                .takes_value(true)
                        )
                        .arg(
                            Arg::new("ssl_iv")
                                .help("The encryption initialization vector to use")
                                .short('v')
                                .takes_value(true)
                        ),
                ),
        );

    let matches = &app.get_matches_from(args);
    match matches.subcommand() {
        Some(("file", matches)) => match matches.subcommand() {
            Some(("encrypt", matches)) => encrypt(matches),
            Some(("decrypt", matches)) => decrypt(matches),
            _ => Ok(()),
        },
        Some(("pack", matches)) => {
            if let Some(mode) = matches.value_of("mode") {
                match set_lifecycle_mode(mode) {
                    Ok(mode) => logger.info(format!("Mode set to {}", mode))?,
                    Err(e) => logger.error("Failed to set lifecycle mode", e)?,
                }
            }
            match matches.subcommand() {
                Some(("detect", matches)) => detect(matches),
                Some(("build", matches)) => build(matches),
                Some(("test", matches)) => test(matches),
                Some(("publish", matches)) => publish(matches),
                Some(_) => Err(anyhow!(
                    "pack subcommand {} not supported",
                    matches.subcommand().unwrap().0
                )),
                _ => Err(anyhow!("pack subcommand missing")),
            }
        }
        Some(_) => Err(anyhow!(
            "Command {} not supported",
            matches.subcommand().unwrap().0
        )),
        _ => Err(anyhow!("cli command missing")),
    }
}

fn init(
    args: &ArgMatches,
    logger: &mut BuildLogger,
) -> (PathBuf, String, PathBuf, PathBuf, Option<PathBuf>) {
    let current_exe = std::env::current_exe().unwrap();
    let current_dir = std::env::current_dir().unwrap();
    let buildpack_dir = current_exe
        .ancestors()
        .find(|a| a.is_dir() && a.join("buildpack.toml").is_file())
        .map(Path::to_path_buf)
        .unwrap();
    let bp_toml = read_file_to_string(buildpack_dir.join("buildpack.toml")).unwrap();

    let app_dir = match args.value_of("source") {
        None => current_dir
            .ancestors()
            .find(|a| a.join("app.toml").is_file())
            .map(Path::to_path_buf),
        Some(s) => Some(PathBuf::from(s)),
    }
    .unwrap();

    let platform_dir = match args.value_of("env") {
        Some(s) => {
            // Heroku-style env dir given.  Add links to simulate CNB style platform dir.
            let target_env_dir = PathBuf::from(s);
            let platform_dir = buildpack_dir.clone();
            let platform_env_dir = platform_dir.join("env");
            if platform_env_dir.exists() {
                logger
                    .debug("Existing platform env dir/link found in buildpack directory.")
                    .unwrap();
            } else {
                logger
                    .debug(format!(
                        "Linking {} to {}.",
                        platform_env_dir.to_string_lossy(),
                        target_env_dir.to_string_lossy()
                    ))
                    .unwrap();
                std::os::unix::fs::symlink(target_env_dir, platform_env_dir).unwrap();
            }
            platform_dir
        }
        None => match args.value_of("platform") {
            Some(s) => PathBuf::from(s),
            None => current_dir.clone(),
        },
    };

    let layers_dir = match args.value_of("layers") {
        None => None,
        Some(s) => Some(PathBuf::from(s)),
    };
    (buildpack_dir, bp_toml, app_dir, platform_dir, layers_dir)
}

fn detect(args: &ArgMatches) -> Result<(), anyhow::Error> {
    let mut logger = BuildLogger::new(true, false);
    logger.header("Pack Detect")?;

    let (buildpack_dir, bp_toml, app_dir, platform_dir, _layers_dir) = init(args, &mut logger);

    let context = DetectContext {
        app_dir: app_dir.to_owned(),
        buildpack_dir: buildpack_dir.to_owned(),
        stack_id: "".to_string(),
        platform: GenericPlatform::from_path(platform_dir).unwrap(),
        buildpack_descriptor: toml::from_str(bp_toml.as_str()).unwrap(),
    };

    match crate::detect(context) {
        Ok(outcome) => match outcome {
            DetectOutcome::Pass(plan) => logger.info(format!(
                "App in {} is suitable for buildpack with plan {:?}",
                &app_dir.to_str().unwrap(),
                plan
            )),
            DetectOutcome::Fail => logger.error(
                "App not suitable",
                anyhow!(
                    "App in {} is not suitable for buildpack",
                    &app_dir.to_str().unwrap()
                ),
            ),
        },
        Err(e) => logger.error("Unexpected error during detect", e),
    }
}

fn build(args: &ArgMatches) -> Result<(), anyhow::Error> {
    let mut logger = BuildLogger::new(true, false);
    logger.header("Pack Build")?;

    let (buildpack_dir, bp_toml, app_dir, platform_dir, layers_dir) = init(args, &mut logger);

    let context = BuildContext {
        layers_dir: match layers_dir {
            Some(path_buf) => path_buf,
            None => Default::default(),
        },
        app_dir: app_dir.to_owned(),
        buildpack_dir: buildpack_dir.to_owned(),
        stack_id: "".to_string(),
        platform: GenericPlatform::from_path(platform_dir).unwrap(),
        buildpack_plan: BuildpackPlan {
            entries: Vec::<Entry>::new(),
        },
        buildpack_descriptor: toml::from_str(bp_toml.as_str()).unwrap(),
    };

    match crate::build(context) {
        Ok(()) => logger.info(format!("Built app in {}", &app_dir.to_str().unwrap())),
        Err(e) => logger.error("Unexpected error during build", e),
    }
}

fn test(args: &ArgMatches) -> Result<(), anyhow::Error> {
    let mut logger = BuildLogger::new(true, false);
    logger.header("Pack Test")?;

    let (buildpack_dir, bp_toml, app_dir, platform_dir, layers_dir) = init(args, &mut logger);

    let context = TestContext {
        layers_dir: match layers_dir {
            Some(path_buf) => path_buf,
            None => Default::default(),
        },
        app_dir: app_dir.to_owned(),
        buildpack_dir: buildpack_dir.to_owned(),
        stack_id: "".to_string(),
        platform: GenericPlatform::from_path(platform_dir).unwrap(),
        buildpack_descriptor: toml::from_str(bp_toml.as_str()).unwrap(),
    };

    match crate::test(context) {
        Ok(outcome) => match outcome {
            TestOutcome::Pass(results) => logger.info(format!(
                "{} tests passed for app in {}",
                results.passed.len(),
                &app_dir.to_str().unwrap()
            )),
            TestOutcome::Fail(results) => logger.error(
                "Tests failed",
                anyhow!(
                    "{} tests failed for app in {}.",
                    results.failed.len(),
                    &app_dir.to_str().unwrap()
                ),
            ),
        },
        Err(e) => logger.error("Unexpected error during test", e),
    }
}

fn publish(args: &ArgMatches) -> Result<(), anyhow::Error> {
    let mut logger = BuildLogger::new(true, false);
    logger.header("Pack Publish")?;

    let (buildpack_dir, bp_toml, app_dir, platform_dir, _layers_dir) = init(args, &mut logger);

    let context = PublishContext {
        app_dir: app_dir.to_owned(),
        buildpack_dir: buildpack_dir.to_owned(),
        stack_id: "".to_string(),
        platform: GenericPlatform::from_path(platform_dir).unwrap(),
        buildpack_descriptor: toml::from_str(bp_toml.as_str()).unwrap(),
    };

    match crate::publish(context) {
        Ok(_) => logger.info(format!("App in {} published", &app_dir.to_str().unwrap())),
        Err(e) => logger.error("Unexpected error during publish", e),
    }
}

fn encrypt(m: &ArgMatches) -> Result<(), anyhow::Error> {
    let mut logger = BuildLogger::new(true, false);
    logger.header("Encrypt File")?;

    let source_file = PathBuf::from(m.value_of("source").unwrap());
    match read_enc(m, source_file) {
        Ok(f) => {
            let target_file = PathBuf::from(m.value_of("target").unwrap());
            enc_file::encrypt(&f, &target_file)?;
            logger.info(format!(
                "File encrypted: {}",
                &target_file.to_str().unwrap()
            ))
        }
        Err(e) => logger.error("Unexpected error during encrypt", e),
    }
}

fn decrypt(m: &ArgMatches) -> Result<(), anyhow::Error> {
    let mut logger = BuildLogger::new(true, false);
    logger.header("Decrypt File")?;

    let source_file = PathBuf::from(m.value_of("source").unwrap());
    match read_enc(m, source_file) {
        Ok(f) => {
            let target_file = PathBuf::from(m.value_of("target").unwrap());
            enc_file::decrypt(&f, &target_file)?;
            logger.info(format!(
                "File decrypted: {}",
                &target_file.to_str().unwrap()
            ))
        }
        Err(e) => logger.error("Unexpected error during decrypt", e),
    }
}

fn read_enc(m: &ArgMatches, source_file: PathBuf) -> Result<EncFile, Error> {
    let ssl_key = match m.value_of("ssl_key") {
        Some(s) => s.to_string(),
        None => match env::var("OPENSSL_ENC_KEY") {
            Ok(key) => key.to_string(),
            Err(_) => {
                return Err(anyhow!(
                    "Requires either ssl_key argument or environment variable OPENSSL_ENC_KEY."
                ));
            }
        },
    };
    let ssl_iv = match m.value_of("ssl_iv") {
        Some(s) => s.to_string(),
        None => match env::var("OPENSSL_ENC_IV") {
            Ok(s) => s.to_string(),
            Err(_) => {
                return Err(anyhow!(
                    "Requires either ssl_iv argument or environment variable OPENSSL_ENC_IV."
                ));
            }
        },
    };
    let enc_file = EncFile::new(&source_file, ssl_key, ssl_iv);
    enc_file
}

#[cfg(test)]
mod tests {
    use crate::cli::execute;
    use crate::util::enc_file::{decrypt, EncFile};
    use libcnb::{read_file_to_string, write_file};
    use std::env;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_file_encrypt() {
        setup_env();

        let temp_dir = tempdir().unwrap();
        let home = temp_dir.as_ref();
        let file = home.join("dummy.key");
        let content = "Whoa this is some content.";
        write_file(content.as_bytes(), &file);

        let enc_file = home.join("dummy.key.enc");

        // Execute
        let args = Vec::from([
            "cli".to_string(),
            "file".to_string(),
            "encrypt".to_string(),
            file.to_str().unwrap().to_string(),
            enc_file.to_str().unwrap().to_string(),
        ]);
        execute(args).unwrap();

        // Check your work
        let unenc_file = home.join("dummy.key.unenc");
        decrypt(&EncFile::from_env(enc_file).unwrap(), &unenc_file).unwrap();
        let text = read_file_to_string(unenc_file).unwrap();
        assert_eq!(content, text.as_str());
    }

    #[test]
    fn test_cli() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let app_dir = root_dir.join("tests/fixtures/sf-package");
        let args = Vec::from([
            "cli".to_string(),
            "pack".to_string(),
            "detect".to_string(),
            app_dir.to_str().unwrap().to_string(),
        ]);
        execute(args).unwrap();
    }

    fn setup_env() {
        env::set_var(
            "OPENSSL_ENC_KEY",
            "C639A572E14D5075C526FDDD43E4ECF6B095EA17783D32EF3D2710AF9F359DD4",
        );
        env::set_var("OPENSSL_ENC_IV", "D09A4D2C5DC39843FE075313A7EF2F4C");
    }
}
