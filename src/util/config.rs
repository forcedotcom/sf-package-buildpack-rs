use json::JsonValue;
use libcnb::read_file_to_string;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use toml::value::Table;

pub fn read_package_directories(
    app_dir: &PathBuf,
    existing: bool,
    shallow: bool,
) -> Option<Vec<PathBuf>> {
    let project_file_json = read_project_file_json(app_dir);

    let mut paths: HashMap<&OsStr, PathBuf> = HashMap::new();
    match &project_file_json["packageDirectories"] {
        json::JsonValue::Array(v) => {
            for e in v {
                match e {
                    json::JsonValue::Object(obj) => match &obj["path"] {
                        json::JsonValue::Short(j) => {
                            let absolute_path = app_dir.join(j.to_string());
                            if !existing || absolute_path.exists() {
                                let path = Path::new(j.as_str());
                                if shallow {
                                    let root = path.iter().next().unwrap();
                                    paths.insert(root, path.to_path_buf().clone());
                                } else {
                                    paths.insert(path.as_os_str(), path.to_path_buf().clone());
                                }
                            }
                        }
                        _ => println!("{:?}", e),
                    },
                    _ => println!("{:?}", e),
                }
            }
            Some(paths.into_iter().map(|(_key, p)| p).collect())
        }
        _ => None,
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SFPackageBuildpackConfig {
    pub runtime: SFDXRuntimeConfig,
}

/// Struct containing the url and sha256 checksum for a downloadable sfdx-runtime-java-runtime.
/// This is used in both `buildpack.toml` and the `layer.toml` but with different keys.
#[derive(Debug, Deserialize, Serialize)]
pub struct SFDXRuntimeConfig {
    pub url: String,
    pub manifest: String,
    pub sha256: String,
}

impl SFDXRuntimeConfig {
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

        SFDXRuntimeConfig {
            url,
            manifest,
            sha256,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SFPackageAppConfig {
    pub default: DefaultConfig,
    pub package: PackageConfig,
    pub dev: DevConfig,
    pub ci: CIConfig,
}

impl Default for SFPackageAppConfig {
    fn default() -> Self {
        let default_config = DefaultConfig::default();

        SFPackageAppConfig {
            default: default_config,
            package: PackageConfig::default(),
            dev: DevConfig::default(),
            ci: CIConfig::default(),
        }
    }
}

impl SFPackageAppConfig {
    pub fn from_dir(app_dir: &PathBuf) -> Self {
        let file = app_dir.join("app.toml");
        if let Ok(file_text) = read_file_to_string(file.as_path()) {
            let mut config: SFPackageAppConfig = toml::from_str(&file_text).unwrap();
            config.package.set_defaults(&config.default);
            config.dev.set_defaults(&config.default);
            config.ci.set_defaults(&config.default);
            config
        } else {
            SFPackageAppConfig::default()
        }
    }
}

#[derive(Deserialize, Debug, Serialize)]
pub struct DefaultConfig {
    #[serde(default)]
    pub hub_client_id: String,
    #[serde(default)]
    pub hub_key: String,
    #[serde(default)]
    pub hub_instance_url: String,
    #[serde(default)]
    pub hub_user: String,
    #[serde(default)]
    pub hub_alias: Option<String>,
    #[serde(default)]
    pub org_def_path: String,
    #[serde(default)]
    pub op_wait_seconds: i32,
}

impl Default for DefaultConfig {
    fn default() -> Self {
        DefaultConfig {
            hub_client_id: "".to_string(),
            hub_key: "".to_string(),
            hub_instance_url: "https://login.salesforce.com".to_string(),
            hub_user: "".to_string(),
            hub_alias: None,
            org_def_path: "config/project-scratch-def.json".to_string(),
            op_wait_seconds: 120,
        }
    }
}

#[derive(Deserialize, Debug, Serialize, Default)]
pub struct PackageConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub namespace: String,
    #[serde(default)]
    pub create_if_needed: bool,
    #[serde(default)]
    pub hub_client_id: String,
    #[serde(default)]
    pub hub_key: String,
    #[serde(default)]
    pub hub_instance_url: String,
    #[serde(default)]
    pub hub_user: String,
    #[serde(default)]
    pub hub_alias: Option<String>,
    #[serde(default)]
    pub org_def_path: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub package_type: String,
    #[serde(default)]
    pub root: String,
    #[serde(default)]
    pub installation_key: String,
    #[serde(default)]
    pub version_name: String,
    #[serde(default)]
    pub version_number: String,
    #[serde(default)]
    pub op_wait_seconds: i32,
}

impl PackageConfig {
    fn set_defaults(&mut self, config: &DefaultConfig) {
        if self.hub_client_id.is_empty() {
            self.hub_client_id = config.hub_client_id.clone();
        }
        if self.hub_key.is_empty() {
            self.hub_key = config.hub_key.clone();
        }
        if self.hub_instance_url.is_empty() {
            self.hub_instance_url = config.hub_instance_url.clone();
        }
        if self.hub_user.is_empty() {
            self.hub_user = config.hub_user.clone();
        }
        if let None = self.hub_alias {
            self.hub_alias = config.hub_alias.clone();
        }
        if self.org_def_path.is_empty() {
            self.org_def_path = config.org_def_path.clone();
        }
        if self.op_wait_seconds <= 0 {
            self.op_wait_seconds = config.op_wait_seconds;
        }
        if self.package_type.is_empty() {
            self.package_type = "Unlocked".to_string();
        }
        if self.root.is_empty() {
            self.root = "force-app".to_string();
        }
    }
}

#[derive(Deserialize, Debug, Serialize, Default)]
pub struct DevConfig {
    #[serde(default)]
    pub hub_client_id: String,
    #[serde(default)]
    pub hub_key: String,
    #[serde(default)]
    pub hub_instance_url: String,
    #[serde(default)]
    pub hub_user: String,
    #[serde(default)]
    pub hub_alias: Option<String>,
    #[serde(default)]
    pub org_def_path: String,
    #[serde(default)]
    pub op_wait_seconds: i32,
    #[serde(default)]
    pub org_alias: String,
    #[serde(default)]
    pub org_duration_days: i32,
    #[serde(default)]
    pub run_tests: bool,
    #[serde(default)]
    pub test_results_path: Option<String>,
    #[serde(default)]
    pub test_results_format: TestResultsFormat,
}

impl DevConfig {
    fn set_defaults(&mut self, config: &DefaultConfig) {
        if self.hub_client_id.is_empty() {
            self.hub_client_id = config.hub_client_id.clone();
        }
        if self.hub_key.is_empty() {
            self.hub_key = config.hub_key.clone();
        }
        if self.hub_user.is_empty() {
            self.hub_user = config.hub_user.clone();
        }
        if let None = self.hub_alias {
            self.hub_alias = config.hub_alias.clone();
        }
        if self.hub_instance_url.is_empty() {
            self.hub_instance_url = config.hub_instance_url.clone();
        }
        if self.org_def_path.is_empty() {
            self.org_def_path = config.org_def_path.clone();
        }
        if self.op_wait_seconds <= 0 {
            self.op_wait_seconds = config.op_wait_seconds;
        }
        if self.org_duration_days <= 0 {
            self.org_duration_days = 7;
        }
    }
}

#[derive(Deserialize, Debug, Serialize, Default)]
pub struct CIConfig {
    #[serde(default)]
    pub hub_instance_url: String,
    #[serde(default)]
    pub hub_client_id: String,
    #[serde(default)]
    pub hub_key: String,
    #[serde(default)]
    pub hub_user: String,
    #[serde(default)]
    pub hub_alias: Option<String>,
    #[serde(default)]
    pub org_def_path: String,
    #[serde(default)]
    pub op_wait_seconds: i32,
    #[serde(default)]
    pub org_alias: String,
    #[serde(default)]
    pub org_duration_days: i32,
    #[serde(default)]
    pub test_results_path: Option<String>,
    #[serde(default)]
    pub test_results_format: TestResultsFormat,
}

impl CIConfig {
    fn set_defaults(&mut self, config: &DefaultConfig) {
        if self.hub_client_id.is_empty() {
            self.hub_client_id = config.hub_client_id.clone();
        }
        if self.hub_key.is_empty() {
            self.hub_key = config.hub_key.clone();
        }
        if self.hub_user.is_empty() {
            self.hub_user = config.hub_user.clone();
        }
        if let None = self.hub_alias {
            self.hub_alias = config.hub_alias.clone();
        }
        if self.hub_instance_url.is_empty() {
            self.hub_instance_url = config.hub_instance_url.clone();
        }
        if self.org_def_path.is_empty() {
            self.org_def_path = config.org_def_path.clone();
        }
        if self.op_wait_seconds <= 0 {
            self.op_wait_seconds = config.op_wait_seconds;
        }
        if self.org_alias.is_empty() {
            self.org_alias = "ci".to_string();
        }
        if self.org_duration_days <= 0 {
            self.org_duration_days = 1;
        }
        if self.test_results_path.is_none() {
            self.test_results_path = Some("test-results".to_string());
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TestResultsFormat {
    Human,
    TAP,
    JUnit,
    JSON,
}

impl Default for TestResultsFormat {
    fn default() -> Self {
        TestResultsFormat::Human
    }
}

impl std::fmt::Display for TestResultsFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TestResultsFormat::Human => "human",
                TestResultsFormat::TAP => "tap",
                TestResultsFormat::JUnit => "junit",
                TestResultsFormat::JSON => "json",
            }
        )
    }
}

fn read_project_file_json(app_dir: &PathBuf) -> JsonValue {
    let project_file = app_dir.join("sfdx-project.json");
    let project_file_text = read_file_to_string(project_file.as_path()).unwrap();
    let project_file_json = json::parse(&project_file_text).unwrap();
    project_file_json
}

#[cfg(test)]
mod tests {
    use crate::util::config;
    use libcnb::write_file;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn setup() -> PathBuf {
        let project_file_content = r#"
{
  "packageDirectories": [
        {
            "path": "force-app",
            "default": true,
            "package": "Test App",
            "versionNumber": "1.0.0.NEXT",
            "definitionFile": "config/project-scratch-def.json"
        },
        {
            "path": "force-app/vendor"
        },
        {
            "path": "force-app/common"
        },
        {
            "path": "force-app/initializer"
        },
        {
            "path": "force-app/postinstall"
        },
        {
            "path": "force-app",
            "package": "Test App Dev",
            "versionNumber": "2.0.0.NEXT",
            "default": false
        },
        {
            "path": "force-app-deux",
            "package": "Test App Deux",
            "versionNumber": "1.0.0.NEXT",
            "default": false
        },
        {
            "path": "force-app-trois",
            "package": "Test App Trois",
            "versionNumber": "1.0.0.NEXT",
            "default": false
        }
    ],
  "namespace": "",
  "sfdcLoginUrl": "https://login.salesforce.com",
  "sourceApiVersion": "52.0"
}
"#;
        let temp_app_dir = tempdir().unwrap().into_path();
        let f = temp_app_dir.join("sfdx-project.json");
        write_file(project_file_content.as_bytes(), &f);

        fs::create_dir(temp_app_dir.join("force-app")).unwrap();
        fs::create_dir(temp_app_dir.join("force-app/vendor")).unwrap();
        fs::create_dir(temp_app_dir.join("force-app/common")).unwrap();
        fs::create_dir(temp_app_dir.join("force-app/initializer")).unwrap();
        fs::create_dir(temp_app_dir.join("force-app/postinstall")).unwrap();

        temp_app_dir
    }

    #[test]
    fn it_should_find_one_root() {
        let app_dir = setup();
        let dirs = config::read_package_directories(&app_dir, true, true);
        match dirs {
            Some(dirs) => {
                assert_eq!(dirs.len(), 1);
                assert!(dirs[0].to_string_lossy().ends_with("force-app"));
            }
            _ => panic!("Expected some dirs"),
        };
    }

    #[test]
    fn it_should_find_two_roots() {
        let app_dir = setup();

        // Add additional root dir
        fs::create_dir(app_dir.join("force-app-deux")).unwrap();

        let opt = config::read_package_directories(&app_dir, true, true);
        match opt {
            Some(mut dirs) => {
                dirs.sort();
                assert_eq!(dirs.len(), 2);
                assert!(dirs[0].to_string_lossy().ends_with("force-app"));
                assert!(dirs[1].to_string_lossy().ends_with("force-app-deux"));
            }
            _ => panic!("Expected some dirs"),
        };
    }

    #[test]
    fn it_should_find_five_dirs() {
        let app_dir = setup();

        let dirs = config::read_package_directories(&app_dir, true, false).unwrap();
        assert_eq!(dirs.len(), 5);
    }

    #[test]
    fn it_should_find_seven_dirs() {
        let app_dir = setup();

        let dirs = config::read_package_directories(&app_dir, false, false).unwrap();
        assert_eq!(dirs.len(), 7);
    }
}
