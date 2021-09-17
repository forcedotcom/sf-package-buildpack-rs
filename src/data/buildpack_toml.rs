use crate::data::Runtime;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use toml::value::Table;

/// This struct is the buildpack defined `metadata` key in the `buildpack.toml`.
#[derive(Deserialize, Serialize)]
pub struct SFPackageBuildpackMetadata {
    pub runtime: Runtime,
    pub release: Release,
}

impl TryFrom<&Table> for SFPackageBuildpackMetadata {
    type Error = anyhow::Error;

    fn try_from(value: &Table) -> Result<Self, Self::Error> {
        Ok(toml::from_str(&toml::to_string(&value)?)?)
    }
}

#[derive(Deserialize, Serialize)]
pub struct Release {
    pub docker: Docker,
}

#[derive(Deserialize, Serialize)]
pub struct Docker {
    pub repository: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    #[test]
    fn metadata_try_from_parses_vendored_buildpack_toml() -> anyhow::Result<()> {
        let buildpack_toml: libcnb::data::buildpack::BuildpackToml<toml::value::Table> =
            toml::from_str(&fs::read_to_string(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("buildpack.toml"),
            )?)?;

        assert!(SFPackageBuildpackMetadata::try_from(&buildpack_toml.metadata).is_ok());

        let metadata = SFPackageBuildpackMetadata::try_from(&buildpack_toml.metadata)?;
        println!("{}", metadata.release.docker.repository);

        Ok(())
    }
}
