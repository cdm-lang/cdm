use super::*;
use serial_test::serial;

#[test]
fn test_parse_template_registry_json() {
    let json = r#"{
        "version": 1,
        "updated_at": "2026-01-07T00:00:00Z",
        "templates": {
            "sql-types": {
                "description": "SQL type definitions for CDM schemas (PostgreSQL, SQLite)",
                "repository": "git:https://github.com/cdm-lang/cdm.git",
                "official": true,
                "versions": {
                    "1.0.0": {
                        "download_url": "https://github.com/cdm-lang/cdm/releases/download/sql-types-v1.0.0/sql-types-1.0.0.tar.gz",
                        "checksum": "sha256:abc123"
                    }
                },
                "latest": "1.0.0"
            }
        }
    }"#;

    let registry: TemplateRegistry = serde_json::from_str(json).unwrap();
    assert_eq!(registry.version, 1);
    assert_eq!(registry.templates.len(), 1);

    let template = registry.templates.get("sql-types").unwrap();
    assert_eq!(template.description, "SQL type definitions for CDM schemas (PostgreSQL, SQLite)");
    assert!(template.official);
    assert_eq!(template.latest, "1.0.0");

    let version = template.versions.get("1.0.0").unwrap();
    assert!(version.download_url.contains("sql-types"));
    assert_eq!(version.checksum, "sha256:abc123");
}

#[test]
#[serial]
fn test_get_template_registry_url_default() {
    let saved = std::env::var("CDM_TEMPLATE_REGISTRY_URL").ok();

    unsafe {
        std::env::remove_var("CDM_TEMPLATE_REGISTRY_URL");
    }

    let url = get_template_registry_url();
    assert!(url.contains("github.com") || url.contains("cdm"));
    assert!(url.contains("templates.json"));

    if let Some(val) = saved {
        unsafe {
            std::env::set_var("CDM_TEMPLATE_REGISTRY_URL", val);
        }
    }
}

#[test]
#[serial]
fn test_get_template_registry_url_custom() {
    unsafe {
        std::env::set_var("CDM_TEMPLATE_REGISTRY_URL", "https://custom-registry.com/templates.json");
    }
    let url = get_template_registry_url();
    assert_eq!(url, "https://custom-registry.com/templates.json");
    unsafe {
        std::env::remove_var("CDM_TEMPLATE_REGISTRY_URL");
    }
}

#[test]
fn test_template_registry_serialization_round_trip() {
    let mut templates = HashMap::new();
    templates.insert(
        "test/template".to_string(),
        RegistryTemplate {
            description: "A test template".to_string(),
            repository: "https://github.com/test/template".to_string(),
            official: true,
            versions: {
                let mut v = HashMap::new();
                v.insert(
                    "1.0.0".to_string(),
                    RegistryTemplateVersion {
                        download_url: "https://github.com/test/template/releases/download/v1.0.0/test-1.0.0.tar.gz".to_string(),
                        checksum: "sha256:abc123".to_string(),
                    },
                );
                v
            },
            latest: "1.0.0".to_string(),
        },
    );

    let registry = TemplateRegistry {
        version: 1,
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        templates,
    };

    let json = serde_json::to_string(&registry).unwrap();
    let deserialized: TemplateRegistry = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.version, 1);
    assert_eq!(deserialized.templates.len(), 1);
    assert!(deserialized.templates.contains_key("test/template"));
}

#[test]
fn test_template_version_parsing() {
    let json = r#"{
        "download_url": "https://github.com/test/repo/releases/download/v1.0.0/template-1.0.0.tar.gz",
        "checksum": "sha256:def456"
    }"#;

    let version: RegistryTemplateVersion = serde_json::from_str(json).unwrap();
    assert!(version.download_url.contains("v1.0.0"));
    assert_eq!(version.checksum, "sha256:def456");
}

#[test]
fn test_template_with_multiple_versions() {
    let json = r#"{
        "description": "Multi-version template",
        "repository": "https://github.com/test/repo",
        "official": false,
        "versions": {
            "1.0.0": {
                "download_url": "https://example.com/v1.0.0.tar.gz",
                "checksum": "sha256:v1hash"
            },
            "2.0.0": {
                "download_url": "https://example.com/v2.0.0.tar.gz",
                "checksum": "sha256:v2hash"
            },
            "2.1.0": {
                "download_url": "https://example.com/v2.1.0.tar.gz",
                "checksum": "sha256:v21hash"
            }
        },
        "latest": "2.1.0"
    }"#;

    let template: RegistryTemplate = serde_json::from_str(json).unwrap();
    assert_eq!(template.versions.len(), 3);
    assert_eq!(template.latest, "2.1.0");
    assert!(!template.official);
}

#[test]
fn test_lookup_template() {
    let mut templates = HashMap::new();
    templates.insert(
        "sql-types".to_string(),
        RegistryTemplate {
            description: "SQL type definitions".to_string(),
            repository: "https://github.com/test".to_string(),
            official: true,
            versions: HashMap::new(),
            latest: "1.0.0".to_string(),
        },
    );

    let registry = TemplateRegistry {
        version: 1,
        updated_at: "2026-01-01".to_string(),
        templates,
    };

    assert!(lookup_template(&registry, "sql-types").is_some());
    assert!(lookup_template(&registry, "nonexistent").is_none());
}

#[test]
fn test_get_template_version() {
    let mut versions = HashMap::new();
    versions.insert(
        "1.0.0".to_string(),
        RegistryTemplateVersion {
            download_url: "https://example.com/v1.0.0.tar.gz".to_string(),
            checksum: "sha256:v1hash".to_string(),
        },
    );
    versions.insert(
        "2.0.0".to_string(),
        RegistryTemplateVersion {
            download_url: "https://example.com/v2.0.0.tar.gz".to_string(),
            checksum: "sha256:v2hash".to_string(),
        },
    );

    let template = RegistryTemplate {
        description: "Test".to_string(),
        repository: "https://github.com/test".to_string(),
        official: true,
        versions,
        latest: "2.0.0".to_string(),
    };

    // Get specific version
    let v1 = get_template_version(&template, Some("1.0.0"));
    assert!(v1.is_some());
    assert!(v1.unwrap().download_url.contains("v1.0.0"));

    // Get latest version when None
    let latest = get_template_version(&template, None);
    assert!(latest.is_some());
    assert!(latest.unwrap().download_url.contains("v2.0.0"));

    // Get nonexistent version
    let nonexistent = get_template_version(&template, Some("3.0.0"));
    assert!(nonexistent.is_none());
}

#[test]
fn test_load_template_registry_with_valid_cache() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path();

    let registry_file = cache_dir.join("templates.json");
    let meta_file = cache_dir.join("templates.meta.json");

    let test_registry = TemplateRegistry {
        version: 1,
        updated_at: "2026-01-01".to_string(),
        templates: HashMap::new(),
    };

    fs::write(&registry_file, serde_json::to_string(&test_registry).unwrap()).unwrap();

    let now = current_timestamp();
    let valid_meta = TemplateRegistryMeta {
        fetched_at: now,
        expires_at: now + 86400,
    };
    fs::write(&meta_file, serde_json::to_string(&valid_meta).unwrap()).unwrap();

    let result = load_template_registry_with_cache_path(cache_dir);
    assert!(result.is_ok());

    let registry = result.unwrap();
    assert_eq!(registry.version, 1);
    assert_eq!(registry.updated_at, "2026-01-01");
}

#[test]
fn test_load_template_registry_cache_miss() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();

    let result = load_template_registry_with_cache_path(temp_dir.path());

    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("fetch") || error_msg.contains("HTTP") || error_msg.contains("registry"),
            "Unexpected error: {}",
            error_msg
        );
    }
}
