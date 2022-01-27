use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Output};

use libcnb::layer_lifecycle::execute_layer_lifecycle;
use libcnb::{
    find_one_file, write_file, BuildContext, GenericPlatform, PlatformEnv, TestOutcome, TestResult,
    TestResults, TestStatus,
};

use crate::util::config::{read_package_directories, SFPackageBuildpackConfig, TestResultsFormat};
use crate::{BuildLogger, Logger};

use crate::layers::sfdx::SFDXLayerLifecycle;
use crate::util::config;
use crate::util::enc_file::{decrypt, EncFile};
use anyhow::anyhow;
use std::io::{BufRead, BufReader};
use std::str::FromStr;

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

//{
//   "status": 0,
//   "result": {
//     "id": "00D3t000004SKHiEAO",
//     "accessToken": "00D3t000004SKHi!ARcAQJr1zmx8VGxeTaUmSevec9XwFd3jvbCIuM0ctdpG_WJ1jStEye9E__TeIziZJBocoLvSr7Z91pNgtVQv4Tj6Akc_SVET",
//     "instanceUrl": "https://mphhub-dev-ed.my.salesforce.com",
//     "username": "mhoefer@mphhub.org",
//     "clientId": "3MVG9JEx.BE6yifMwrjHPgoh5LBDEECZgHw9odyBrMZ4.qsQI_CqDLjnQDkPFjVOsuzCoAHuaAS9Sd0TqnTJG",
//     "connectedStatus": "Connected"
//   },
//   "warnings": [
//     "This command will expose sensitive information that allows for subsequent activity using your current authenticated session.\nSharing this information is equivalent to logging someone in under the current credential, resulting in unintended access and escalation of privilege.\nFor additional information, please review the authorization section of the https://developer.salesforce.com/docs/atlas.en-us.234.0.sfdx_dev.meta/sfdx_dev/sfdx_dev_auth_web_flow.htm"
//   ]
// }
#[derive(Serialize, Deserialize, Debug)]
pub struct OrgDisplay {
    status: i32,
    result: Option<OrgDisplayResult>,
    warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrgDisplayResult {
    pub id: String,
    pub access_token: String,
    pub instance_url: String,
    pub username: String,
    pub client_id: String,
    pub connected_status: Option<OrgStatus>,
    pub status: Option<OrgStatus>,
}

/*
{
  "status": 0,
  "result": {
    "Id": "08c3t000000Xa2kAAC",
    "Status": "Success",
    "Package2Id": "0Ho3t000000XZNrCAO",
    "Package2VersionId": "05i3t000000XZeMAAW",
    "SubscriberPackageVersionId": "04t3t000002zQrEAAU",
    "Tag": null,
    "Branch": null,
    "Error": [],
    "CreatedDate": "2022-01-05 11:38",
    "HasMetadataRemoved": false,
    "CreatedBy": "mhoefer@mphhub.org"
  }
}
 */
#[derive(Serialize, Deserialize, Debug)]
pub struct PackageVersionCreate {
    status: i32,
    result: PackageVersionCreateResult,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PackageVersionCreateResult {
    pub id: String,
    pub status: String,
    pub package2_id: String,
    pub package2_version_id: String,
    pub subscriber_package_version_id: String,
}

/*
{
  "status": 0,
  "result": {
    "attributes": {
      "type": "Package2Version",
      "url": "/services/data/v53.0/tooling/sobjects/Package2Version/05i3t000000XZdxAAG"
    },
    "Package2Id": "0Ho3t000000XZNrCAO",
    "SubscriberPackageVersionId": "04t3t000002zQqpAAE",
    "Name": "Version One",
    "Description": null,
    "Tag": null,
    "Branch": null,
    "AncestorId": "N/A",
    "ValidationSkipped": false,
    "MajorVersion": 1,
    "MinorVersion": 0,
    "PatchVersion": 0,
    "BuildNumber": 2,
    "IsReleased": false,
    "CodeCoverage": null,
    "HasPassedCodeCoverageCheck": false,
    "Package2": {
      "attributes": {
        "type": "Package2",
        "url": "/services/data/v53.0/tooling/sobjects/Package2/0Ho3t000000XZNrCAO"
      },
      "IsOrgDependent": "No"
    },
    "ReleaseVersion": 53,
    "BuildDurationInSeconds": 60,
    "HasMetadataRemoved": "N/A",
    "CreatedBy": "mhoefer@mphhub.org",
    "Version": "1.0.0.2",
    "AncestorVersion": "N/A"
  }
}
 */
#[derive(Serialize, Deserialize, Debug)]
pub struct PackageVersion {
    status: i32,
    result: PackageVersionResult,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PackageVersionResult {
    pub package2_id: String,
    pub subscriber_package_version_id: String,
    pub name: String,
    pub version: String,
    pub ancestor_version: String,
    pub is_released: bool,
}

#[derive(Deserialize, Debug, Serialize)]
pub enum OrgStatus {
    Active,
    Deleted,
    Connected,
    Disconnected,
    ENOENT,
}

impl FromStr for OrgStatus {
    type Err = anyhow::Error;

    fn from_str(status: &str) -> Result<OrgStatus, Self::Err> {
        match status {
            "Connected" => Ok(OrgStatus::Connected),
            "Disconnected" => Ok(OrgStatus::Disconnected),
            _ => Err(anyhow!("Invalid status string")),
        }
    }
}

pub fn sfdx_display_org(
    layers_dir: &PathBuf,
    app_dir: &PathBuf,
    user: &str,
) -> Option<OrgDisplayResult> {
    let mut cmd = sfdx(layers_dir);
    match cmd
        .args(vec!["force:org:display", "-u", user, "--json"])
        .output()
    {
        Ok(output) => {
            let s = std::str::from_utf8(output.stdout.as_slice()).unwrap();
            let res: OrgDisplay = serde_json::from_str(s).unwrap();
            res.result
        }
        Err(e) => {
            panic!(
                "failed to execute {:?} from {:?} due to {}",
                cmd, app_dir, e
            );
        }
    }
}

pub fn sfdx_check_org(layers_dir: &PathBuf, app_dir: &PathBuf, user: &str) -> Option<OrgStatus> {
    if let Some(org_info) = sfdx_display_org(layers_dir, app_dir, user) {
        if let Some(status) = org_info.connected_status {
            Some(status)
        } else if let Some(status) = org_info.status {
            Some(status)
        } else {
            None
        }
    } else {
        None
    }
}

fn sfdx(layers_dir: &PathBuf) -> Command {
    let sfdx_bin = layers_dir.join("sfdx").join("bin");
    config::append_local_env_path(sfdx_bin);

    Command::new("sfdx")
}

pub fn sfdx_auth(
    layers_dir: &PathBuf,
    app_dir: &PathBuf,
    client_id: &str,
    key_path: &str,
    instance_url: &str,
    user_name: &str,
    alias: Option<String>,
    env: &PlatformEnv,
) -> Result<(), anyhow::Error> {
    let mut logger = BuildLogger::new(true, true);

    // Exit early if we are already authenticated.
    if env.var("SFDX_AUTH_FORCE").is_ok() {
        logger.info("re-authenticating hub")?;
    } else if let Some(OrgStatus::Connected) = sfdx_check_org(layers_dir, app_dir, user_name) {
        logger.info("Hub already authenticated")?;
        return Ok(());
    }

    let key_file = match env.var("SFDX_AUTH_KEYFILE") {
        Ok(s) => {
            // Try the KEYFILE var first
            let p = PathBuf::from(s);
            if p.is_file() {
                logger.info(format!("found SFDX_AUTH_KEYFILE {}", p.to_str().unwrap()))?;
                Ok(p)
            } else {
                Err(anyhow!("Location given but no such file exists"))
            }
        }
        Err(_) => match env.var("SFDX_AUTH_ENC_KEYFILE") {
            Ok(s) => {
                // Try the ENC_KEYFILE var next
                let p = PathBuf::from(s);
                logger.info(format!(
                    "found SFDX_AUTH_ENC_KEYFILE {}",
                    p.to_str().unwrap()
                ))?;
                decrypt_key(layers_dir, &mut logger, p, env)
            }
            Err(_) => {
                // Lastly, try the configured key file value
                let mut p = PathBuf::from(key_path);
                if p.is_relative() {
                    p = app_dir.join(p);
                }
                logger.info(format!(
                    "trying key_path app config value {}",
                    p.to_str().unwrap()
                ))?;
                decrypt_key(layers_dir, &mut logger, p, env)
            }
        },
    };

    let url_file = match env.var("SFDX_AUTH_URLFILE") {
        Ok(s) => {
            let p = PathBuf::from(s);
            if p.is_file() {
                logger.info(format!("found SFDX_AUTH_URLFILE {}", p.to_str().unwrap()))?;
                Ok(p)
            } else {
                // Location given but no such file exists
                Err(anyhow!("Location given but no such file exists"))
            }
        }
        Err(_) => match env.var("SFDX_AUTH_URL") {
            Ok(s) => {
                let p = layers_dir.join("sfdx").join(".sfdx_auth_url");
                write_file(s.as_bytes(), &p);
                logger.info(format!("found SFDX_AUTH_URL {}", s))?;
                Ok(p)
            }
            Err(_) => Err(anyhow!("No auth url or urlfile provided")),
        },
    };

    let access_token = env.var("SFDX_ACCESS_TOKEN");
    if access_token.is_ok() {
        logger.info("found SFDX_ACCESS_TOKEN")?;
    }

    if let Ok(key_file) = key_file {
        logger.info("authenticating hub with key")?;
        let mut cmd = sfdx(layers_dir);
        cmd.current_dir(app_dir)
            .arg("auth:jwt:grant")
            .arg("--clientid")
            .arg(client_id)
            .arg("--jwtkeyfile")
            .arg(key_file.canonicalize().unwrap())
            .arg("--username")
            .arg(user_name)
            .arg("--instanceurl")
            .arg(instance_url)
            .arg("--setdefaultdevhubusername");
        if let Some(s) = alias {
            logger.info(format!("using alias {}", &s))?;
            cmd.arg("--setalias").arg(s);
        }
        match cmd.output() {
            Ok(output) => {
                logger.output("authenticated hub", output)?;
                Ok(())
            }
            Err(e) => Err(anyhow::Error::new(e)),
        }
    } else if let Ok(url_file) = url_file {
        logger.info("authenticating hub with url")?;
        let mut cmd = sfdx(layers_dir);
        match cmd
            .current_dir(app_dir)
            .arg("auth:sfdxurl:store")
            .arg("-f")
            .arg(url_file.canonicalize().unwrap())
            .arg("--setdefaultdevhubusername")
            .output()
        {
            Ok(output) => {
                logger.output("authenticated hub", output)?;
                Ok(())
            }
            Err(e) => Err(anyhow::Error::new(e)),
        }
    } else if let Ok(access_token) = access_token {
        logger.info("authenticating hub with token")?;
        let mut cmd = sfdx(layers_dir);
        match cmd
            .current_dir(app_dir)
            .env("SFDX_ACCESS_TOKEN", access_token)
            .arg("auth:accesstoken:store")
            .arg("--instanceurl")
            .arg(instance_url)
            .arg("--setdefaultdevhubusername")
            .arg("--noprompt")
            .output()
        {
            Ok(output) => {
                logger.output("authenticated hub", output)?;
                Ok(())
            }
            Err(e) => Err(anyhow::Error::new(e)),
        }
    } else {
        Err(anyhow!("Unable to authenticate hub.  Hub should be pre-authenticated, \
        or one of SFDX_AUTH_KEYFILE, SFDX_AUTH_ENC_KEYFILE, SFDX_AUTH_URL, SFDX_AUTH_URLFILE, or SFDX_ACCESS_TOKEN must be provided."))
    }
}

fn decrypt_key(
    layers_dir: &PathBuf,
    logger: &mut BuildLogger,
    p: PathBuf,
    env: &PlatformEnv,
) -> Result<PathBuf, anyhow::Error> {
    let enc_file = EncFile::new(p, env.var("OPENSSL_ENC_KEY")?, env.var("OPENSSL_ENC_IV")?);
    if enc_file.is_ok() {
        logger.info("found SFDX_AUTH_ENC_KEYFILE, OPENSSL_ENC_KEY and OPENSSL_ENC_IV")?;
        let sfdx_dir = layers_dir.join("sfdx");
        fs::create_dir_all(&sfdx_dir)?;
        let target_file = sfdx_dir.join(".sfdx_auth_key");
        decrypt(&enc_file.unwrap(), &target_file)?;
        logger.info(format!(
            "Decrypted file to {}",
            &target_file.to_str().unwrap()
        ))?;
        Ok(target_file)
    } else {
        Err(anyhow!("Location given but no such file exists"))
    }
}

pub fn sfdx_create_org_if_needed(
    layers_dir: &PathBuf,
    app_dir: &PathBuf,
    hub_user: &str,
    scratch_org_def_path: &str,
    scratch_org_duration: i32,
    scratch_org_alias: &str,
    logger: &mut BuildLogger,
) -> Result<bool, anyhow::Error> {
    let created = match sfdx_check_org(layers_dir, app_dir, scratch_org_alias) {
        Some(OrgStatus::Active) => false,
        _ => {
            logger.info("---> Creating scratch org")?;
            let output = sfdx_create_org(
                layers_dir,
                app_dir,
                hub_user,
                scratch_org_def_path,
                scratch_org_duration,
                scratch_org_alias,
            )?;
            logger.output("creating environment", output)?;
            true
        }
    };
    Ok(created)
}

pub fn sfdx_create_org(
    layers_dir: &PathBuf,
    app_dir: &PathBuf,
    hub_user: &str,
    scratch_org_def_path: &str,
    scratch_org_duration: i32,
    scratch_org_alias: &str,
) -> Result<Output, anyhow::Error> {
    let mut cmd = sfdx(layers_dir);
    cmd.current_dir(app_dir)
        .arg("force:org:create")
        .arg("-v")
        .arg(hub_user)
        .arg("-f")
        .arg(scratch_org_def_path)
        .arg("-d")
        .arg(scratch_org_duration.to_string())
        .arg("-a")
        .arg(scratch_org_alias);
    match cmd.output() {
        Ok(output) => {
            let status = output.status.code().unwrap();
            if status != 0 {
                let stderr = String::from_utf8(output.stderr.to_owned()).unwrap();
                Err(anyhow::anyhow!(
                    "failed to execute {:?} from {:?}:\n{}",
                    cmd,
                    app_dir,
                    stderr
                ))
            } else {
                Ok(output)
            }
        }
        Err(e) => Err(anyhow::anyhow!(
            "failed to execute {:?} from {:?} due to {}",
            cmd,
            app_dir,
            e
        )),
    }
}

pub fn sfdx_delete_org(
    layers_dir: &PathBuf,
    app_dir: &PathBuf,
    hub_user: &str,
    scratch_org_alias: &str,
) -> Result<Output, anyhow::Error> {
    let mut cmd = sfdx(layers_dir);
    cmd.current_dir(app_dir)
        .arg("force:org:delete")
        .arg("-v")
        .arg(hub_user)
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
                    hub_user,
                    scratch_org_alias,
                    stderr
                ));
            }
            Ok(output)
        }
        Err(e) => {
            eprintln!(
                "failed to delete scratch org on {} named {}",
                hub_user, scratch_org_alias
            );
            Err(anyhow::anyhow!(e))
        }
    }
}

pub fn sfdx_push_source(
    layers_dir: &PathBuf,
    app_dir: &PathBuf,
    scratch_org_alias: &str,
    wait_seconds: i32,
) -> Result<Output, anyhow::Error> {
    let mut cmd = sfdx(layers_dir);
    let mut child = cmd
        .current_dir(app_dir)
        .arg("force:source:push")
        .arg("-f")
        .arg("-u")
        .arg(scratch_org_alias)
        .arg("-w")
        .arg(wait_seconds.to_string())
        .spawn()
        .expect("failed to execute command");

    output_stderr(&mut child);

    let output = child.wait_with_output().expect("failed to wait on child");
    if output.status.success() {
        Ok(output)
    } else {
        Err(anyhow::anyhow!(
            "failed to push source to {}:\n Exited with {}",
            scratch_org_alias,
            output.status.code().unwrap()
        ))
    }
}

pub struct SfdxResponse<R> {
    pub status: u8,
    pub result: R,
}

pub struct CreatePackageResult {
    pub created: bool,
    pub package_id: String,
}

pub struct FindPackageResult {
    pub package_id: String,
}

pub fn sfdx_find_package(
    layers_dir: &PathBuf,
    app_dir: &PathBuf,
    hub_user: &String,
    package_name: &String,
) -> Result<SfdxResponse<FindPackageResult>, anyhow::Error> {
    let mut cmd = sfdx(layers_dir);
    let output = cmd
        .current_dir(app_dir)
        .arg("force:package:list")
        .arg("--json")
        .arg("-v")
        .arg(hub_user)
        .output()
        .expect("failed to execute command");

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        let v: serde_json::Value = serde_json::from_str(stdout.as_str())?;
        let package_values = v["result"].as_array().unwrap();
        match package_values
            .iter()
            .find(|v| v["Name"].as_str().unwrap().eq(package_name))
        {
            Some(package) => Ok(SfdxResponse {
                status: 0,
                result: FindPackageResult {
                    package_id: package["Id"].as_str().unwrap().to_string(),
                },
            }),
            None => Ok(SfdxResponse {
                status: 1,
                result: FindPackageResult {
                    package_id: "".to_string(),
                },
            }),
        }
    } else {
        Err(anyhow::anyhow!(
            "failed to create new package {}:\n Exited with {}",
            package_name,
            output.status.code().unwrap()
        ))
    }
}

pub fn sfdx_create_package(
    layers_dir: &PathBuf,
    app_dir: &PathBuf,
    hub_user: &String,
    package_name: &String,
    package_desc: &String,
    package_type: &String,
    package_root: &String,
) -> Result<SfdxResponse<CreatePackageResult>, anyhow::Error> {
    let mut cmd = sfdx(layers_dir);
    let mut child = cmd
        .current_dir(app_dir)
        .arg("force:package:create")
        .arg("--json")
        .arg("-v")
        .arg(hub_user)
        .arg("-n")
        .arg(package_name)
        .arg("-d")
        .arg(package_desc)
        .arg("-t")
        .arg(package_type)
        .arg("-r")
        .arg(package_root)
        .spawn()
        .expect("failed to execute command");

    output_stderr(&mut child);

    let output = child.wait_with_output().expect("failed to wait on command");
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        let v: serde_json::Value = serde_json::from_str(stdout.as_str())?;
        let status = 0;
        let result = CreatePackageResult {
            created: true,
            package_id: v["result"]["Id"].to_string(),
        };
        Ok(SfdxResponse { status, result })
    } else {
        Err(anyhow::anyhow!(
            "failed to create new package {}:\n Exited with {}",
            package_name,
            output.status.code().unwrap()
        ))
    }
}

fn output_stderr(child: &mut Child) {
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        reader
            .lines()
            .filter_map(|line| line.ok())
            .for_each(|line| eprintln!("{}", line));
    }
}

pub fn sfdx_create_package_version(
    layers_dir: &PathBuf,
    app_dir: &PathBuf,
    hub_user: &String,
    package_id: &String,
    org_def_path: &String,
    version_name: &String,
    version_number: &String,
    installation_key: &String,
    wait_seconds: i32,
) -> Result<PackageVersionResult, anyhow::Error> {
    let mut cmd = sfdx(layers_dir);
    cmd.current_dir(&app_dir)
        .arg("force:package:version:create")
        .arg("--json")
        .arg("-p")
        .arg(package_id)
        .arg("-v")
        .arg(hub_user)
        .arg("-f")
        .arg(org_def_path)
        .arg("-a")
        .arg(version_name)
        .arg("-n")
        .arg(version_number)
        .arg("-w")
        .arg(wait_seconds.to_string());
    if installation_key.is_empty() {
        cmd.arg("-x");
    } else {
        cmd.arg("-k").arg(installation_key);
    }
    let output = cmd.output().expect("failed to execute command");

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        let v: PackageVersionCreate = serde_json::from_str(stdout.as_str())?;
        let id = v.result.subscriber_package_version_id; // 04t...
        sfdx_fetch_package_version(layers_dir, app_dir, hub_user, &id)
    } else {
        let stdout = String::from_utf8(output.stdout)?;
        let details: serde_json::Value = serde_json::from_str(stdout.as_str())?;
        Err(anyhow::anyhow!(
            "failed to create new package version of {}\n{}: {}",
            package_id,
            details["name"].as_str().unwrap(),
            details["message"].as_str().unwrap(),
        ))
    }
}

pub fn sfdx_fetch_package_version(
    layers_dir: &PathBuf,
    app_dir: &PathBuf,
    hub_user: &String,
    id: &String,
) -> Result<PackageVersionResult, anyhow::Error> {
    let mut cmd = sfdx(layers_dir);
    cmd.current_dir(&app_dir)
        .arg("force:package:version:report")
        .arg("--json")
        .arg("-p")
        .arg(id)
        .arg("-v")
        .arg(hub_user);
    let output = cmd.output().expect("failed to execute command");

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        let v: PackageVersion = serde_json::from_str(stdout.as_str())?;
        Ok(v.result)
    } else {
        let stdout = String::from_utf8(output.stdout)?;
        let details: serde_json::Value = serde_json::from_str(stdout.as_str())?;
        Err(anyhow::anyhow!(
            "failed to fetch package version {}\n{}: {}",
            id,
            details["name"].as_str().unwrap(),
            details["message"].as_str().unwrap(),
        ))
    }
}

/* {
    "status": 0,
    "result": ApexTestRunResult
} */
#[derive(Serialize, Deserialize, Debug)]
pub struct ApexTestRun {
    pub status: i32,
    pub result: ApexTestRunResult,
}

/* {
    "summary": ApexTestSummary,
    "tests": [ ApexTestResult ],
} */
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApexTestRunResult {
    pub summary: ApexTestSummary,
    pub tests: Vec<ApexTestResult>,
}

impl Into<TestOutcome> for ApexTestRunResult {
    fn into(self) -> TestOutcome {
        let mut results = TestResults::new();
        for test in self.tests {
            match test.outcome {
                ApexTestOutcome::Pass => results
                    .passed
                    .push(TestResult::new(test.full_name, TestStatus::Pass)),
                ApexTestOutcome::Fail => results
                    .failed
                    .push(TestResult::new(test.full_name, TestStatus::Fail)),
                ApexTestOutcome::Ignore => results
                    .ignored
                    .push(TestResult::new(test.full_name, TestStatus::Ignore)),
            }
        }
        match self.summary.outcome {
            ApexTestSummaryOutcome::Passed => TestOutcome::Pass(results),
            ApexTestSummaryOutcome::Failed => TestOutcome::Fail(results),
        }
    }
}
/* {
  "outcome": "Passed",
  "testsRan": 2,
  "passing": 2,
  "failing": 0,
  "skipped": 0,
  "passRate": "100%",
  "failRate": "0%",
  "testStartTime": "Thu Jan 06 2022 2:44:54 PM",
  "testExecutionTime": "24 ms",
  "testTotalTime": "24 ms",
  "commandTime": "244 ms",
  "hostname": "https://velocity-energy-3793-dev-ed.cs77.my.salesforce.com",
  "orgId": "00D0t000000MeWZEA0",
  "username": "test-ahmet6briymu@example.com",
  "testRunId": "7070t00001vpgqx",
  "userId": "0050t000009C5sDAAS",
  "testRunCoverage": "100%",
  "orgWideCoverage": "100%"
} */
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApexTestSummary {
    pub outcome: ApexTestSummaryOutcome,
    pub tests_ran: i32,
    pub passing: i32,
    pub failing: i32,
    pub skipped: i32,
    pub pass_rate: String,
    pub fail_rate: String,
    pub test_start_time: String,
    pub test_execution_time: String,
    pub test_total_time: String,
    pub command_time: String,
    pub hostname: String,
    pub org_id: String,
    pub username: String,
    pub test_run_id: String,
    pub user_id: String,
    pub test_run_coverage: String,
    pub org_wide_coverage: String,
}

/*
{
    "Id": "07M0t00000FfffwEAB",
    "QueueItemId": "7090t0000022UUlAAM",
    "StackTrace": null,
    "Message": null,
    "AsyncApexJobId": "7070t00001vpgqxAAA",
    "MethodName": "testBehavior",
    "Outcome": "Pass",
    "ApexClass": {
      "Id": "01p0t00000FKeStAAL",
      "Name": "TestTests",
      "NamespacePrefix": null
    },
    "RunTime": 11,
    "FullName": "TestTests.testBehavior"
}
 */
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ApexTestResult {
    pub id: String,
    pub queue_item_id: String,
    pub stack_trace: Option<String>,
    pub message: Option<String>,
    pub async_apex_job_id: String,
    pub method_name: String,
    pub outcome: ApexTestOutcome,
    pub apex_class: ApexTestClass,
    pub run_time: i32,
    pub full_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ApexTestSummaryOutcome {
    Passed,
    Failed,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ApexTestOutcome {
    Pass,
    Fail,
    Ignore,
}

/*{
    Id: 01p0t00000FKeStAAL,
    Name: TestTests,
    NamespacePrefix: null
}*/
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ApexTestClass {
    id: String,
    name: String,
    namespace_prefix: Option<String>,
}

pub fn sfdx_test_apex(
    layers_dir: &PathBuf,
    app_dir: &PathBuf,
    scratch_org_alias: &str,
    results_path: Option<String>,
    results_format: TestResultsFormat,
    wait_seconds: i32,
) -> Result<ApexTestRunResult, anyhow::Error> {
    let mut cmd = sfdx(layers_dir);
    cmd.current_dir(app_dir)
        .arg("force:apex:test:run")
        .arg("-u")
        .arg(scratch_org_alias)
        .arg("-l")
        .arg("RunLocalTests")
        .arg("-w")
        .arg(wait_seconds.to_string())
        .arg("--json")
        .arg("-r")
        .arg(results_format.to_string())
        .arg("-c")
        .arg("-v");
    if let Some(path) = results_path {
        cmd.arg("-d").arg(app_dir.join(path));
    }

    match cmd.output() {
        Ok(output) => {
            let status = output.status.code().unwrap();
            let stdout = String::from_utf8(output.stdout)?;
            let stderr = String::from_utf8(output.stderr)?;
            // This is a Hack, to work around the platform bug that throws an error when no apex tests exist.
            if status != 0
                && !stderr
                    .contains("Always provide a classes, suites, tests, or testLevel property")
            {
                return Err(anyhow::anyhow!(
                    "failed to run apex tests on {}:\n {}",
                    scratch_org_alias,
                    stderr
                ));
            }
            let result: ApexTestRun = serde_json::from_str(stdout.as_str())?;
            Ok(result.result)
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
    use std::path::PathBuf;
    use std::{env, fs};
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
}
