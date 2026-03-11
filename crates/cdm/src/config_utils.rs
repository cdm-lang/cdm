use std::path::PathBuf;

/// Extract output paths from a config value that may be a string or array of strings.
/// Returns an empty Vec if the key is missing or has no valid paths.
pub(crate) fn extract_output_paths(config: &serde_json::Value, key: &str) -> Vec<PathBuf> {
    match config.get(key) {
        Some(serde_json::Value::String(s)) if !s.is_empty() => vec![PathBuf::from(s)],
        Some(serde_json::Value::Array(arr)) => {
            arr.iter()
                .filter_map(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .collect()
        }
        _ => vec![],
    }
}

#[cfg(test)]
#[path = "config_utils/config_utils_tests.rs"]
mod config_utils_tests;
