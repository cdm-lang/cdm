use super::*;
use std::collections::HashMap;

fn make_versions(vers: &[&str]) -> HashMap<String, RegistryVersion> {
    vers.iter()
        .map(|v| {
            (
                v.to_string(),
                RegistryVersion {
                    wasm_url: format!("https://example.com/{}.wasm", v),
                    checksum: "sha256:test".to_string(),
                },
            )
        })
        .collect()
}

#[test]
fn test_parse_version_constraint_exact() {
    let constraint = parse_version_constraint("1.2.3").unwrap();
    matches!(constraint, VersionConstraint::Exact(_));
}

#[test]
fn test_parse_version_constraint_caret() {
    let constraint = parse_version_constraint("^1.2.3").unwrap();
    if let VersionConstraint::Caret(v) = constraint {
        assert_eq!(v, "1.2.3");
    } else {
        panic!("Expected Caret variant");
    }
}

#[test]
fn test_parse_version_constraint_tilde() {
    let constraint = parse_version_constraint("~1.2.3").unwrap();
    if let VersionConstraint::Tilde(v) = constraint {
        assert_eq!(v, "1.2.3");
    } else {
        panic!("Expected Tilde variant");
    }
}

#[test]
fn test_parse_version_constraint_range() {
    let constraint = parse_version_constraint(">=1.0.0 <2.0.0").unwrap();
    if let VersionConstraint::Range(min, max) = constraint {
        assert_eq!(min, "1.0.0");
        assert_eq!(max, "2.0.0");
    } else {
        panic!("Expected Range variant");
    }
}

#[test]
fn test_parse_version_constraint_latest() {
    let constraint = parse_version_constraint("latest").unwrap();
    matches!(constraint, VersionConstraint::Latest);

    let constraint2 = parse_version_constraint("").unwrap();
    matches!(constraint2, VersionConstraint::Latest);
}

#[test]
fn test_resolve_exact_version() {
    let versions = make_versions(&["1.0.0", "1.1.0", "2.0.0"]);
    let constraint = VersionConstraint::Exact("1.1.0".to_string());
    let result = resolve_version(&constraint, &versions);
    assert_eq!(result, Some("1.1.0".to_string()));
}

#[test]
fn test_resolve_exact_version_not_found() {
    let versions = make_versions(&["1.0.0", "1.1.0"]);
    let constraint = VersionConstraint::Exact("2.0.0".to_string());
    let result = resolve_version(&constraint, &versions);
    assert_eq!(result, None);
}

#[test]
fn test_resolve_latest_version() {
    let versions = make_versions(&["1.0.0", "1.1.0", "2.0.0"]);
    let constraint = VersionConstraint::Latest;
    let result = resolve_version(&constraint, &versions);
    assert_eq!(result, Some("2.0.0".to_string()));
}

#[test]
fn test_resolve_caret_constraint() {
    let versions = make_versions(&["1.0.0", "1.1.0", "1.2.0", "2.0.0"]);
    let constraint = VersionConstraint::Caret("1.0.0".to_string());
    let result = resolve_version(&constraint, &versions);
    // ^1.0.0 should match highest 1.x
    assert_eq!(result, Some("1.2.0".to_string()));
}

#[test]
fn test_resolve_tilde_constraint() {
    let versions = make_versions(&["1.0.0", "1.0.1", "1.0.2", "1.1.0"]);
    let constraint = VersionConstraint::Tilde("1.0.0".to_string());
    let result = resolve_version(&constraint, &versions);
    // ~1.0.0 should match highest 1.0.x
    assert_eq!(result, Some("1.0.2".to_string()));
}

#[test]
fn test_resolve_range_constraint() {
    let versions = make_versions(&["0.9.0", "1.0.0", "1.5.0", "2.0.0", "3.0.0"]);
    let constraint = VersionConstraint::Range("1.0.0".to_string(), "2.0.0".to_string());
    let result = resolve_version(&constraint, &versions);
    // >=1.0.0 <2.0.0 should match 1.5.0
    assert_eq!(result, Some("1.5.0".to_string()));
}

#[test]
fn test_display_constraint() {
    assert_eq!(VersionConstraint::Exact("1.0.0".to_string()).to_string(), "1.0.0");
    assert_eq!(VersionConstraint::Caret("1.0.0".to_string()).to_string(), "^1.0.0");
    assert_eq!(VersionConstraint::Tilde("1.0.0".to_string()).to_string(), "~1.0.0");
    assert_eq!(
        VersionConstraint::Range("1.0.0".to_string(), "2.0.0".to_string()).to_string(),
        ">=1.0.0 <2.0.0"
    );
    assert_eq!(VersionConstraint::Latest.to_string(), "latest");
}
