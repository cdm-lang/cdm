use super::*;
use serial_test::serial;

#[test]
fn test_parse_template_registry_json() {
    let json = r#"{
        "version": 1,
        "updated_at": "2026-01-07T00:00:00Z",
        "templates": {
            "sql/postgres-types": {
                "description": "PostgreSQL type definitions",
                "repository": "git:https://github.com/cdm-lang/cdm.git",
                "official": true,
                "versions": {
                    "1.0.0": {
                        "git_url": "https://github.com/cdm-lang/cdm.git",
                        "git_ref": "main",
                        "git_path": "templates/sql/postgres-types"
                    }
                },
                "latest": "1.0.0"
            }
        }
    }"#;

    let registry: TemplateRegistry = serde_json::from_str(json).unwrap();
    assert_eq!(registry.version, 1);
    assert_eq!(registry.templates.len(), 1);

    let template = registry.templates.get("sql/postgres-types").unwrap();
    assert_eq!(template.description, "PostgreSQL type definitions");
    assert!(template.official);
    assert_eq!(template.latest, "1.0.0");

    let version = template.versions.get("1.0.0").unwrap();
    assert_eq!(version.git_ref, "main");
    assert_eq!(version.git_path, Some("templates/sql/postgres-types".to_string()));
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
                        git_url: "https://github.com/test/template.git".to_string(),
                        git_ref: "v1.0.0".to_string(),
                        git_path: Some("templates/test".to_string()),
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
fn test_template_version_without_git_path() {
    let json = r#"{
        "git_url": "https://github.com/test/repo.git",
        "git_ref": "v1.0.0"
    }"#;

    let version: RegistryTemplateVersion = serde_json::from_str(json).unwrap();
    assert_eq!(version.git_url, "https://github.com/test/repo.git");
    assert_eq!(version.git_ref, "v1.0.0");
    assert!(version.git_path.is_none());
}

#[test]
fn test_template_with_multiple_versions() {
    let json = r#"{
        "description": "Multi-version template",
        "repository": "https://github.com/test/repo",
        "official": false,
        "versions": {
            "1.0.0": {
                "git_url": "https://github.com/test/repo.git",
                "git_ref": "v1.0.0"
            },
            "2.0.0": {
                "git_url": "https://github.com/test/repo.git",
                "git_ref": "v2.0.0"
            },
            "2.1.0": {
                "git_url": "https://github.com/test/repo.git",
                "git_ref": "v2.1.0"
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
        "sql/postgres-types".to_string(),
        RegistryTemplate {
            description: "PostgreSQL types".to_string(),
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

    assert!(lookup_template(&registry, "sql/postgres-types").is_some());
    assert!(lookup_template(&registry, "nonexistent").is_none());
}

#[test]
fn test_get_template_version() {
    let mut versions = HashMap::new();
    versions.insert(
        "1.0.0".to_string(),
        RegistryTemplateVersion {
            git_url: "https://github.com/test.git".to_string(),
            git_ref: "v1.0.0".to_string(),
            git_path: None,
        },
    );
    versions.insert(
        "2.0.0".to_string(),
        RegistryTemplateVersion {
            git_url: "https://github.com/test.git".to_string(),
            git_ref: "v2.0.0".to_string(),
            git_path: None,
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
    assert_eq!(v1.unwrap().git_ref, "v1.0.0");

    // Get latest version when None
    let latest = get_template_version(&template, None);
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().git_ref, "v2.0.0");

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
