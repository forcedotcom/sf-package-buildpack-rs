use std::collections::HashMap;
use std::path::Path;

use anyhow::Error;
use libcnb::data::layer_content_metadata::LayerContentMetadata;
use libcnb::layer_lifecycle::{LayerLifecycle, ValidateResult};
use libcnb::{BuildContext, GenericPlatform};
use std::env;

use crate::data::buildpack_toml::{Docker, Release, SFPackageBuildpackMetadata};
use crate::data::Runtime;
use crate::util::fetch::{extract, get};

pub struct SFDXLayerLifecycle;

impl
    LayerLifecycle<
        GenericPlatform,
        SFPackageBuildpackMetadata,
        SFPackageBuildpackMetadata,
        HashMap<String, String>,
        anyhow::Error,
    > for SFDXLayerLifecycle
{
    fn create(
        &self,
        layer_path: &Path,
        build_context: &BuildContext<GenericPlatform, SFPackageBuildpackMetadata>,
    ) -> Result<LayerContentMetadata<SFPackageBuildpackMetadata>, anyhow::Error> {
        let runtime_sha256 = extract(
            &build_context.buildpack_descriptor.metadata.runtime.url,
            &layer_path,
            Some("sfdx/"),
        )?;

        Ok(LayerContentMetadata {
            launch: true,
            build: false,
            cache: true,
            metadata: SFPackageBuildpackMetadata {
                runtime: Runtime {
                    url: build_context
                        .buildpack_descriptor
                        .metadata
                        .runtime
                        .url
                        .clone(),
                    manifest: build_context
                        .buildpack_descriptor
                        .metadata
                        .runtime
                        .manifest
                        .clone(),
                    sha256: runtime_sha256.clone(),
                },
                release: Release {
                    docker: Docker {
                        repository: "".to_string(),
                    },
                },
            },
        })
    }

    fn validate(
        &self,
        _layer_path: &Path,
        layer_content_metadata: &LayerContentMetadata<SFPackageBuildpackMetadata>,
        build_context: &BuildContext<GenericPlatform, SFPackageBuildpackMetadata>,
    ) -> ValidateResult {
        // Fetch most recent sfdx.tar.xz manifest
        let manifest_content =
            get(&build_context.buildpack_descriptor.metadata.runtime.manifest).unwrap();
        let manifest = json::parse(&manifest_content).unwrap();

        let manifest_sha256 = match &manifest["sha256xz"] {
            json::JsonValue::String(s) => s,
            _ => "",
        };

        if layer_content_metadata.metadata.runtime.sha256 == manifest_sha256 {
            ValidateResult::KeepLayer
        } else {
            ValidateResult::RecreateLayer
        }
    }

    fn layer_lifecycle_data(
        &self,
        layer_path: &Path,
        _layer_content_metadata: LayerContentMetadata<SFPackageBuildpackMetadata>,
    ) -> Result<HashMap<String, String>, Error> {
        let mut layer_env: HashMap<String, String> = HashMap::new();
        let bin_path = format!("{}/bin", &layer_path.to_str().unwrap());

        layer_env.insert(
            String::from("PATH"),
            format!(
                "{}:{}:{}",
                layer_path.join("bin").as_path().to_str().unwrap(),
                bin_path,
                env::var("PATH").unwrap_or(String::new()),
            ),
        );

        Ok(layer_env)
    }
}

#[cfg(test)]
mod tests {
    use libcnb::data::buildpack::BuildpackToml;
    use libcnb::data::buildpack_plan::BuildpackPlan;
    use libcnb::{BuildContext, GenericPlatform, Platform};
    use std::fs;
    use std::path::PathBuf;
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

    #[test]
    fn test_if_runtime_exists_and_checksum_match() {
        // TODO need mocking to avoid outbound calls
    }

    #[test]
    fn test_if_checksum_does_not_match() {
        // TODO need mocking to avoid outbound calls
    }

    #[test]
    fn test_if_runtime_is_missing() {
        // TODO need mocking to avoid outbound calls
    }
}
