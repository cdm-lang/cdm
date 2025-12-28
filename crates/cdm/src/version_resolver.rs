use anyhow::Result;
use semver::{Version, VersionReq};
use std::collections::HashMap;

use crate::registry::RegistryVersion;

/// Version constraint types
#[derive(Debug, Clone)]
pub enum VersionConstraint {
    Exact(String),           // "1.2.3"
    Caret(String),           // "^1.2.3" (compatible updates)
    Tilde(String),           // "~1.2.3" (patch updates)
    Range(String, String),   // ">=1.0.0 <2.0.0"
    Latest,                  // No constraint specified
}

impl std::fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionConstraint::Exact(v) => write!(f, "{}", v),
            VersionConstraint::Caret(v) => write!(f, "^{}", v),
            VersionConstraint::Tilde(v) => write!(f, "~{}", v),
            VersionConstraint::Range(min, max) => write!(f, ">={} <{}", min, max),
            VersionConstraint::Latest => write!(f, "latest"),
        }
    }
}

/// Parse a version constraint string
pub fn parse_version_constraint(s: &str) -> Result<VersionConstraint> {
    let s = s.trim();

    if s.is_empty() || s == "latest" {
        return Ok(VersionConstraint::Latest);
    }

    if s.starts_with('^') {
        return Ok(VersionConstraint::Caret(s[1..].to_string()));
    }

    if s.starts_with('~') {
        return Ok(VersionConstraint::Tilde(s[1..].to_string()));
    }

    if s.contains(">=") && s.contains('<') {
        // Parse range ">=1.0.0 <2.0.0"
        let parts: Vec<&str> = s.split_whitespace().collect();

        let min = parts
            .iter()
            .find(|p| p.starts_with(">="))
            .and_then(|p| p.strip_prefix(">="))
            .ok_or_else(|| anyhow::anyhow!("Invalid range format: missing '>=' part"))?;

        let max = parts
            .iter()
            .find(|p| p.starts_with('<'))
            .and_then(|p| p.strip_prefix('<'))
            .ok_or_else(|| anyhow::anyhow!("Invalid range format: missing '<' part"))?;

        return Ok(VersionConstraint::Range(min.to_string(), max.to_string()));
    }

    // Exact version
    Ok(VersionConstraint::Exact(s.to_string()))
}

/// Resolve a version constraint to a specific version
pub fn resolve_version(
    constraint: &VersionConstraint,
    available_versions: &HashMap<String, RegistryVersion>,
) -> Option<String> {
    match constraint {
        VersionConstraint::Latest => {
            // Return highest version
            find_highest_version(available_versions)
        }

        VersionConstraint::Exact(ver) => {
            // Check if exact version exists
            if available_versions.contains_key(ver) {
                Some(ver.clone())
            } else {
                None
            }
        }

        VersionConstraint::Caret(ver) | VersionConstraint::Tilde(ver) => {
            let req_str = match constraint {
                VersionConstraint::Caret(_) => format!("^{}", ver),
                VersionConstraint::Tilde(_) => format!("~{}", ver),
                _ => unreachable!(),
            };

            match VersionReq::parse(&req_str) {
                Ok(req) => find_matching_version(&req, available_versions),
                Err(_) => None,
            }
        }

        VersionConstraint::Range(min, max) => {
            let req_str = format!(">={}, <{}", min, max);
            match VersionReq::parse(&req_str) {
                Ok(req) => find_matching_version(&req, available_versions),
                Err(_) => None,
            }
        }
    }
}

/// Find the highest version available
fn find_highest_version(available_versions: &HashMap<String, RegistryVersion>) -> Option<String> {
    let mut versions: Vec<Version> = available_versions
        .keys()
        .filter_map(|v| Version::parse(v).ok())
        .collect();

    if versions.is_empty() {
        return None;
    }

    versions.sort();
    versions.last().map(|v| v.to_string())
}

/// Find the highest version matching a requirement
fn find_matching_version(
    req: &VersionReq,
    available_versions: &HashMap<String, RegistryVersion>,
) -> Option<String> {
    let mut matching: Vec<Version> = available_versions
        .keys()
        .filter_map(|v| Version::parse(v).ok())
        .filter(|v| req.matches(v))
        .collect();

    if matching.is_empty() {
        return None;
    }

    matching.sort();
    matching.last().map(|v| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
