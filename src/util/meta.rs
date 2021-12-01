use std::path::PathBuf;
use std::str::FromStr;
use libcnb::{read_file_to_string, TomlFileError, write_toml_file};
use serde::{Serialize, Deserialize};
use chrono;
use chrono::TimeZone;
use toml::value::Datetime;

#[derive(Deserialize, Debug, Serialize)]
pub enum PackageVersionStatus {
    Beta,
    Published,
}

impl FromStr for PackageVersionStatus {
    type Err = anyhow::Error;

    fn from_str(status: &str) -> Result<PackageVersionStatus, Self::Err> {
        match status {
            "Published" => Ok(PackageVersionStatus::Published),
            "Beta" => Ok(PackageVersionStatus::Beta),
            _ => Err(anyhow::Error::msg("Invalid status string")),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct SFPackageAppMeta {
    package_versions: Vec<PackageVersionMeta>,
    package: PackageMeta,
}

impl SFPackageAppMeta {
    pub fn from_dir(app_dir: &PathBuf) -> Self {
        let file = app_dir.join("app-meta.toml");
        if let Some(file_text) = read_file_to_string(file.as_path()) {
            toml::from_str(&file_text).unwrap()
        } else {
            SFPackageAppMeta::default()
        }
    }

    pub fn to_dir(&self, app_dir: &PathBuf) -> Result<(), TomlFileError> {
        let file = app_dir.join("app-meta.toml");
        write_toml_file(self, file)
    }
}

#[derive(Deserialize, Debug, Serialize, Default)]
pub struct PackageMeta {
    id: String,
    name: String,
    hub_alias: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PackageVersionMeta {
    id: String,
    name: String,
    number: String,
    package_id: String,
    created_date: Datetime,
    status: PackageVersionStatus,
}

pub fn write_package_meta(app_dir: &PathBuf, id: &String, name: &String, hub_alias: &String)
        -> Result<(), anyhow::Error> {
    let mut app_meta = SFPackageAppMeta::from_dir(app_dir);
    app_meta.package = PackageMeta {
        id: id.to_string(),
        name: name.to_string(),
        hub_alias: hub_alias.to_string()
    };
    match app_meta.to_dir(app_dir) {
        Ok(()) => Ok(()),
        Err(e) => Err(anyhow::Error::new(e))
    }
}

pub fn write_package_version_meta(app_dir: &PathBuf, name: &String, number: &String, package_id: &String,
                                  id: String, created_date: String) -> Result<(), anyhow::Error> {
    let mut app_meta = SFPackageAppMeta::from_dir(app_dir);
    app_meta.package_versions.push(PackageVersionMeta {
        id,
        name: name.to_string(),
        number: number.to_string(),
        package_id: package_id.to_string(),
        created_date: to_toml_datetime(created_date.as_str()),
        status: PackageVersionStatus::Beta
    });
    match app_meta.to_dir(app_dir) {
        Ok(()) => Ok(()),
        Err(e) => Err(anyhow::Error::new(e))
    }
}

// From format: 2021-11-19 11:48
// to format: 0000-00-00T00:00:00.00
fn to_toml_datetime(str: &str) -> Datetime {
    match chrono::Utc.datetime_from_str(str, "%Y-%m-%d %H:%M") {
        Ok(d) => {
            let toml_str = d.format("%Y-%m-%dT%H:%M:00").to_string();
            Datetime::from_str(toml_str.as_str()).unwrap()
        },
        Err(e) => panic!("{}", e)
    }
}
