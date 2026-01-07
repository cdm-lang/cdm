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
        "https://raw.githubusercontent.com/cdm-lang/cdm/refs/heads/main/cli-releases.json"
            .to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // DESERIALIZATION TESTS
    // =========================================================================

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

    #[test]
    fn test_manifest_multiple_releases() {
        let json = r#"{
            "version": 1,
            "updated_at": "2025-12-30T00:00:00Z",
            "latest": "0.3.0",
            "releases": {
                "0.1.0": {
                    "release_date": "2025-12-01",
                    "platforms": {}
                },
                "0.2.0": {
                    "release_date": "2025-12-15",
                    "platforms": {}
                },
                "0.3.0": {
                    "release_date": "2025-12-30",
                    "platforms": {}
                }
            }
        }"#;

        let manifest: CliReleaseManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.releases.len(), 3);
        assert!(manifest.releases.contains_key("0.1.0"));
        assert!(manifest.releases.contains_key("0.2.0"));
        assert!(manifest.releases.contains_key("0.3.0"));
    }

    #[test]
    fn test_manifest_multiple_platforms() {
        let json = r#"{
            "version": 1,
            "updated_at": "2025-12-29T00:00:00Z",
            "latest": "0.2.0",
            "releases": {
                "0.2.0": {
                    "release_date": "2025-12-29",
                    "platforms": {
                        "x86_64-apple-darwin": {
                            "url": "https://example.com/cdm-macos-x64",
                            "checksum": "sha256:abc123"
                        },
                        "aarch64-apple-darwin": {
                            "url": "https://example.com/cdm-macos-arm",
                            "checksum": "sha256:def456"
                        },
                        "x86_64-unknown-linux-gnu": {
                            "url": "https://example.com/cdm-linux-x64",
                            "checksum": "sha256:ghi789"
                        }
                    }
                }
            }
        }"#;

        let manifest: CliReleaseManifest = serde_json::from_str(json).unwrap();
        let release = manifest.releases.get("0.2.0").unwrap();
        assert_eq!(release.platforms.len(), 3);
        assert!(release.platforms.contains_key("x86_64-apple-darwin"));
        assert!(release.platforms.contains_key("aarch64-apple-darwin"));
        assert!(release.platforms.contains_key("x86_64-unknown-linux-gnu"));
    }

    #[test]
    fn test_manifest_empty_releases() {
        let json = r#"{
            "version": 1,
            "updated_at": "2025-12-29T00:00:00Z",
            "latest": "",
            "releases": {}
        }"#;

        let manifest: CliReleaseManifest = serde_json::from_str(json).unwrap();
        assert!(manifest.releases.is_empty());
    }

    #[test]
    fn test_platform_release_fields() {
        let json = r#"{
            "url": "https://cdn.example.com/releases/cdm-v1.0.0-darwin.tar.gz",
            "checksum": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        }"#;

        let release: PlatformRelease = serde_json::from_str(json).unwrap();
        assert_eq!(release.url, "https://cdn.example.com/releases/cdm-v1.0.0-darwin.tar.gz");
        assert!(release.checksum.starts_with("sha256:"));
    }

    #[test]
    fn test_release_fields() {
        let json = r#"{
            "release_date": "2025-12-29",
            "platforms": {
                "x86_64-apple-darwin": {
                    "url": "https://example.com/cdm",
                    "checksum": "sha256:abc"
                }
            }
        }"#;

        let release: Release = serde_json::from_str(json).unwrap();
        assert_eq!(release.release_date, "2025-12-29");
        assert_eq!(release.platforms.len(), 1);
    }

    // =========================================================================
    // SERIALIZATION TESTS
    // =========================================================================

    #[test]
    fn test_manifest_serialization() {
        let mut platforms = HashMap::new();
        platforms.insert(
            "x86_64-apple-darwin".to_string(),
            PlatformRelease {
                url: "https://example.com/cdm".to_string(),
                checksum: "sha256:abc".to_string(),
            },
        );

        let mut releases = HashMap::new();
        releases.insert(
            "1.0.0".to_string(),
            Release {
                release_date: "2025-12-29".to_string(),
                platforms,
            },
        );

        let manifest = CliReleaseManifest {
            version: 1,
            updated_at: "2025-12-29T00:00:00Z".to_string(),
            latest: "1.0.0".to_string(),
            releases,
        };

        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("\"version\":1"));
        assert!(json.contains("\"latest\":\"1.0.0\""));
    }

    #[test]
    fn test_roundtrip_serialization() {
        let mut platforms = HashMap::new();
        platforms.insert(
            "x86_64-apple-darwin".to_string(),
            PlatformRelease {
                url: "https://example.com/cdm".to_string(),
                checksum: "sha256:abc123".to_string(),
            },
        );

        let mut releases = HashMap::new();
        releases.insert(
            "1.0.0".to_string(),
            Release {
                release_date: "2025-12-29".to_string(),
                platforms,
            },
        );

        let original = CliReleaseManifest {
            version: 2,
            updated_at: "2025-12-30T12:00:00Z".to_string(),
            latest: "1.0.0".to_string(),
            releases,
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: CliReleaseManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(original.version, deserialized.version);
        assert_eq!(original.latest, deserialized.latest);
        assert_eq!(original.updated_at, deserialized.updated_at);
    }

    // =========================================================================
    // DEBUG AND CLONE TESTS
    // =========================================================================

    #[test]
    fn test_manifest_debug() {
        let manifest = CliReleaseManifest {
            version: 1,
            updated_at: "2025-12-29T00:00:00Z".to_string(),
            latest: "1.0.0".to_string(),
            releases: HashMap::new(),
        };

        let debug = format!("{:?}", manifest);
        assert!(debug.contains("CliReleaseManifest"));
        assert!(debug.contains("version: 1"));
        assert!(debug.contains("latest"));
    }

    #[test]
    fn test_manifest_clone() {
        let manifest = CliReleaseManifest {
            version: 1,
            updated_at: "2025-12-29T00:00:00Z".to_string(),
            latest: "1.0.0".to_string(),
            releases: HashMap::new(),
        };

        let cloned = manifest.clone();
        assert_eq!(cloned.version, manifest.version);
        assert_eq!(cloned.latest, manifest.latest);
        assert_eq!(cloned.updated_at, manifest.updated_at);
    }

    #[test]
    fn test_release_debug() {
        let release = Release {
            release_date: "2025-12-29".to_string(),
            platforms: HashMap::new(),
        };

        let debug = format!("{:?}", release);
        assert!(debug.contains("Release"));
        assert!(debug.contains("release_date"));
    }

    #[test]
    fn test_release_clone() {
        let mut platforms = HashMap::new();
        platforms.insert(
            "test".to_string(),
            PlatformRelease {
                url: "url".to_string(),
                checksum: "checksum".to_string(),
            },
        );

        let release = Release {
            release_date: "2025-12-29".to_string(),
            platforms,
        };

        let cloned = release.clone();
        assert_eq!(cloned.release_date, release.release_date);
        assert_eq!(cloned.platforms.len(), release.platforms.len());
    }

    #[test]
    fn test_platform_release_debug() {
        let release = PlatformRelease {
            url: "https://example.com".to_string(),
            checksum: "sha256:abc".to_string(),
        };

        let debug = format!("{:?}", release);
        assert!(debug.contains("PlatformRelease"));
        assert!(debug.contains("url"));
        assert!(debug.contains("checksum"));
    }

    #[test]
    fn test_platform_release_clone() {
        let release = PlatformRelease {
            url: "https://example.com".to_string(),
            checksum: "sha256:abc".to_string(),
        };

        let cloned = release.clone();
        assert_eq!(cloned.url, release.url);
        assert_eq!(cloned.checksum, release.checksum);
    }

    // =========================================================================
    // MANIFEST URL TESTS
    // =========================================================================

    #[test]
    fn test_get_manifest_url_returns_string() {
        // Test that get_manifest_url returns a non-empty string
        // We can't reliably test env var behavior in parallel tests
        let url = get_manifest_url();
        assert!(!url.is_empty());
        // URL should be a valid HTTPS URL
        assert!(url.starts_with("https://"));
    }

    #[test]
    fn test_get_manifest_url_contains_json() {
        let url = get_manifest_url();
        // The URL should point to a JSON file
        assert!(url.ends_with(".json"));
    }
}
