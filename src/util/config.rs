use crate::util::files::read_file_to_string;
use json::JsonValue;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

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

fn read_project_file_json(app_dir: &PathBuf) -> JsonValue {
    let project_file = app_dir.join("sfdx-project.json");
    let project_file_text = read_file_to_string(project_file.as_path()).unwrap();
    let project_file_json = json::parse(&project_file_text).unwrap();
    project_file_json
}

#[cfg(test)]
mod tests {
    use crate::util::config;
    use crate::util::files;
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
        files::tests::write_file(project_file_content, &f).unwrap();

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
