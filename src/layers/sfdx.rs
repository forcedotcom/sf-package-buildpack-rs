use std::collections::HashMap;
use std::path::Path;

use anyhow::Error;

use libcnb::data::layer_content_metadata::LayerContentMetadata;
use libcnb::layer_lifecycle::{LayerLifecycle, ValidateResult};
use libcnb::{get, get_and_extract, BuildContext, GenericPlatform};
use std::env;

use crate::util::config::{SFDXRuntimeConfig, SFPackageBuildpackConfig};

pub(crate) struct SFDXLayerLifecycle;

impl
    LayerLifecycle<
        GenericPlatform,
        SFPackageBuildpackConfig,
        SFPackageBuildpackConfig,
        HashMap<String, String>,
        anyhow::Error,
    > for SFDXLayerLifecycle
{
    fn create(
        &self,
        layer_path: &Path,
        build_context: &BuildContext<GenericPlatform, SFPackageBuildpackConfig>,
    ) -> Result<LayerContentMetadata<SFPackageBuildpackConfig>, anyhow::Error> {
        let runtime_sha256 = get_and_extract(
            &build_context.buildpack_descriptor.metadata.runtime.url,
            &layer_path,
            Some("sfdx/"),
        )?;

        Ok(LayerContentMetadata::default()
            .build(false)
            .cache(true)
            .launch(true)
            .metadata(SFPackageBuildpackConfig {
                runtime: SFDXRuntimeConfig {
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
            }))
    }

    fn validate(
        &self,
        _layer_path: &Path,
        layer_content_metadata: &LayerContentMetadata<SFPackageBuildpackConfig>,
        build_context: &BuildContext<GenericPlatform, SFPackageBuildpackConfig>,
    ) -> ValidateResult {
        // Get most recent sfdx.tar.xz manifest
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
        _layer_content_metadata: LayerContentMetadata<SFPackageBuildpackConfig>,
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
