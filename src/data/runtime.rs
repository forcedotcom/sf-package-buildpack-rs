use serde::{Deserialize, Serialize};
use toml::value::Table;

/// Struct containing the url and sha256 checksum for a downloadable sf-fx-runtime-java-runtime.
/// This is used in both `buildpack.toml` and the `layer.toml` but with different keys.
#[derive(Debug, Deserialize, Serialize)]
pub struct Runtime {
    pub url: String,
    pub manifest: String,
    pub sha256: String,
}

impl Runtime {
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

        Runtime {
            url,
            manifest,
            sha256,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml::toml;

    #[test]
    fn from_runtime_layer_parses_filled_values() {
        let toml = toml! {
            runtime_url = "https://foo.com"
            runtime_manifest = "https://foo.com/manifest"
            runtime_sha256 = "ABCDEF"
        };
        let runtime = Runtime::from_runtime_layer(&toml.as_table().unwrap());

        assert_eq!(runtime.url, "https://foo.com");
        assert_eq!(runtime.manifest, "https://foo.com/manifest");
        assert_eq!(runtime.sha256, "ABCDEF");
    }

    #[test]
    fn from_runtime_layer_parses_no_url() {
        let toml = toml! {
            runtime_sha256 = "ABCDEF"
        };
        let runtime = Runtime::from_runtime_layer(&toml.as_table().unwrap());

        assert_eq!(runtime.url, "");
        assert_eq!(runtime.manifest, "");
        assert_eq!(runtime.sha256, "ABCDEF");
    }

    #[test]
    fn from_runtime_layer_parses_no_sha256() {
        let toml = toml! {
            runtime_url = "https://foo.com"
        };
        let runtime = Runtime::from_runtime_layer(&toml.as_table().unwrap());

        assert_eq!(runtime.url, "https://foo.com");
        assert_eq!(runtime.sha256, "");
    }
}
