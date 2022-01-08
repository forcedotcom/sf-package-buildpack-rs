use anyhow::anyhow;
use libcnb::{read_file_to_string, write_toml_file, TomlFileError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

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
            _ => Err(anyhow!("Invalid status string")),
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
        if let Ok(file_text) = read_file_to_string(file.as_path()) {
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
    hub_user: String,
    hub_instance_url: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PackageVersionMeta {
    id: String,
    name: String,
    number: String,
    package_id: String,
    status: PackageVersionStatus,
}

pub fn write_package_meta(
    app_dir: &PathBuf,
    id: &String,
    name: &String,
    hub_user: &String,
    hub_instance_url: &String,
) -> Result<(), anyhow::Error> {
    let mut app_meta = SFPackageAppMeta::from_dir(app_dir);
    app_meta.package = PackageMeta {
        id: id.to_string(),
        name: name.to_string(),
        hub_user: hub_user.to_string(),
        hub_instance_url: hub_instance_url.to_string(),
    };
    match app_meta.to_dir(app_dir) {
        Ok(()) => Ok(()),
        Err(e) => Err(anyhow::Error::new(e)),
    }
}

pub fn write_package_version_meta(
    app_dir: &PathBuf,
    id: String,
    package_id: String,
    name: String,
    number: String,
) -> Result<(), anyhow::Error> {
    let mut app_meta = SFPackageAppMeta::from_dir(app_dir);
    app_meta.package_versions.push(PackageVersionMeta {
        id,
        package_id: package_id.to_string(),
        name: name.to_string(),
        number: number.to_string(),
        status: PackageVersionStatus::Beta,
    });
    match app_meta.to_dir(app_dir) {
        Ok(()) => Ok(()),
        Err(e) => Err(anyhow::Error::new(e)),
    }
}
