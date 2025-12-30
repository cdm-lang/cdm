use crate::self_update::error::UpdateError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CLI release manifest structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CliReleaseManifest {
    pub version: u32,
    pub updated_at: String,
    pub latest: String,
    pub releases: HashMap<String, Release>,
}

/// Individual release information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Release {
    pub release_date: String,
    pub platforms: HashMap<String, PlatformRelease>,
}

/// Platform-specific release information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlatformRelease {
    pub url: String,
    pub checksum: String,
}

/// Fetch the CLI release manifest from GitHub
pub fn fetch_manifest() -> Result<CliReleaseManifest, UpdateError> {
    let manifest_url = get_manifest_url();

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client.get(&manifest_url).send()?;

    if !response.status().is_success() {
        return Err(UpdateError::InvalidManifest(format!(
            "Failed to fetch manifest: HTTP {}",
            response.status()
        )));
    }

    let manifest: CliReleaseManifest = response.json()?;

    Ok(manifest)
}

/// Get the manifest URL from environment or use default
fn get_manifest_url() -> String {
    std::env::var("CDM_CLI_REGISTRY_URL").unwrap_or_else(|_| {
        "https://raw.githubusercontent.com/anthropics/cdm/main/cli-releases.json".to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_deserialization() {
        let json = r#"{
            "version": 1,
            "updated_at": "2025-12-29T00:00:00Z",
            "latest": "0.2.0",
            "releases": {
                "0.2.0": {
                    "release_date": "2025-12-29",
                    "platforms": {
                        "x86_64-apple-darwin": {
                            "url": "https://example.com/cdm",
                            "checksum": "sha256:abc123"
                        }
                    }
                }
            }
        }"#;

        let manifest: CliReleaseManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.latest, "0.2.0");
        assert!(manifest.releases.contains_key("0.2.0"));
    }
}
