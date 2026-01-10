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
    latest_version: Option<&str>,
) -> Option<String> {
    match constraint {
        VersionConstraint::Latest => {
            // Use the latest version from registry if provided, otherwise calculate highest
            if let Some(latest) = latest_version {
                if available_versions.contains_key(latest) {
                    return Some(latest.to_string());
                }
            }
            // Fallback to calculating highest version
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

/// Check if a version string matches a version constraint
pub fn version_matches(constraint: &VersionConstraint, version: &str) -> bool {
    match constraint {
        VersionConstraint::Latest => true,
        VersionConstraint::Exact(v) => version == v,
        VersionConstraint::Caret(v) => {
            let req_str = format!("^{}", v);
            if let (Ok(req), Ok(ver)) = (VersionReq::parse(&req_str), Version::parse(version)) {
                req.matches(&ver)
            } else {
                false
            }
        }
        VersionConstraint::Tilde(v) => {
            let req_str = format!("~{}", v);
            if let (Ok(req), Ok(ver)) = (VersionReq::parse(&req_str), Version::parse(version)) {
                req.matches(&ver)
            } else {
                false
            }
        }
        VersionConstraint::Range(min, max) => {
            let req_str = format!(">={}, <{}", min, max);
            if let (Ok(req), Ok(ver)) = (VersionReq::parse(&req_str), Version::parse(version)) {
                req.matches(&ver)
            } else {
                false
            }
        }
    }
}

/// Compare two version strings, returns Ordering
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    match (Version::parse(a), Version::parse(b)) {
        (Ok(va), Ok(vb)) => va.cmp(&vb),
        (Ok(_), Err(_)) => std::cmp::Ordering::Greater,
        (Err(_), Ok(_)) => std::cmp::Ordering::Less,
        (Err(_), Err(_)) => a.cmp(b),
    }
}

#[cfg(test)]
#[path = "version_resolver/version_resolver_tests.rs"]
mod version_resolver_tests;
