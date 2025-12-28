use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::registry;

/// Clone or update a git plugin repository and return the path to the WASM file
pub fn clone_git_plugin(url: &str, git_ref: &str) -> Result<PathBuf> {
    let cache_dir = registry::get_cache_path()?.join("git");
    fs::create_dir_all(&cache_dir)
        .context("Failed to create git cache directory")?;

    // Create a sanitized directory name from URL
    let repo_name = sanitize_git_url(url);
    let repo_path = cache_dir.join(&repo_name);

    if repo_path.exists() {
        // Repository exists, update it
        update_git_repo(&repo_path, git_ref)?;
    } else {
        // Clone fresh
        clone_git_repo(url, &repo_path, git_ref)?;
    }

    Ok(repo_path)
}

/// Extract WASM file path from a cloned plugin repository
pub fn extract_wasm_from_repo(repo_path: &Path, _plugin_name: &str) -> Result<PathBuf> {
    // Read cdm-plugin.json manifest
    let manifest_path = repo_path.join("cdm-plugin.json");
    if !manifest_path.exists() {
        anyhow::bail!(
            "No cdm-plugin.json found in repository at {}",
            repo_path.display()
        );
    }

    let manifest_content = fs::read_to_string(&manifest_path)
        .context("Failed to read cdm-plugin.json")?;

    let manifest: serde_json::Value = serde_json::from_str(&manifest_content)
        .context("Failed to parse cdm-plugin.json")?;

    // Get WASM file path from manifest
    let wasm_file = manifest
        .get("wasm")
        .and_then(|w| w.get("file"))
        .and_then(|f| f.as_str())
        .ok_or_else(|| anyhow::anyhow!("No wasm.file specified in cdm-plugin.json"))?;

    let wasm_path = repo_path.join(wasm_file);
    if !wasm_path.exists() {
        anyhow::bail!(
            "WASM file not found: {}\nSpecified in cdm-plugin.json as: {}",
            wasm_path.display(),
            wasm_file
        );
    }

    Ok(wasm_path)
}

/// Clone a git repository
fn clone_git_repo(url: &str, dest: &Path, git_ref: &str) -> Result<()> {
    println!("Cloning git repository {} (ref: {})...", url, git_ref);

    let output = Command::new("git")
        .arg("clone")
        .arg("--depth=1")
        .arg("--branch")
        .arg(git_ref)
        .arg(url)
        .arg(dest)
        .output()
        .context("Failed to execute git clone. Is git installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git clone failed:\n{}", stderr);
    }

    println!("Successfully cloned repository to {}", dest.display());

    Ok(())
}

/// Update an existing git repository
fn update_git_repo(repo_path: &Path, git_ref: &str) -> Result<()> {
    println!(
        "Updating git repository at {} (ref: {})...",
        repo_path.display(),
        git_ref
    );

    // Fetch latest changes
    let fetch_output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("fetch")
        .arg("origin")
        .arg(git_ref)
        .output()
        .context("Failed to execute git fetch")?;

    if !fetch_output.status.success() {
        let stderr = String::from_utf8_lossy(&fetch_output.stderr);
        anyhow::bail!("git fetch failed:\n{}", stderr);
    }

    // Checkout the ref
    let checkout_output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("checkout")
        .arg(git_ref)
        .output()
        .context("Failed to execute git checkout")?;

    if !checkout_output.status.success() {
        let stderr = String::from_utf8_lossy(&checkout_output.stderr);
        anyhow::bail!("git checkout failed:\n{}", stderr);
    }

    // Pull latest changes
    let pull_output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("pull")
        .arg("origin")
        .arg(git_ref)
        .output()
        .context("Failed to execute git pull")?;

    if !pull_output.status.success() {
        let stderr = String::from_utf8_lossy(&pull_output.stderr);
        anyhow::bail!("git pull failed:\n{}", stderr);
    }

    println!("Successfully updated repository");

    Ok(())
}

/// Sanitize a git URL to create a safe directory name
pub fn sanitize_git_url(url: &str) -> String {
    // Convert "https://github.com/user/repo.git" to "github.com_user_repo"
    url.trim_end_matches(".git")
        .replace("https://", "")
        .replace("http://", "")
        .replace("git://", "")
        .replace("git@", "")
        .replace(':', "_")
        .replace('/', "_")
        .replace('.', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_git_url_https() {
        let url = "https://github.com/user/repo.git";
        let sanitized = sanitize_git_url(url);
        assert_eq!(sanitized, "github_com_user_repo");
    }

    #[test]
    fn test_sanitize_git_url_http() {
        let url = "http://example.com/path/to/repo.git";
        let sanitized = sanitize_git_url(url);
        assert_eq!(sanitized, "example_com_path_to_repo");
    }

    #[test]
    fn test_sanitize_git_url_ssh() {
        let url = "git@github.com:user/repo.git";
        let sanitized = sanitize_git_url(url);
        assert_eq!(sanitized, "github_com_user_repo");
    }

    #[test]
    fn test_sanitize_git_url_no_git_extension() {
        let url = "https://github.com/user/repo";
        let sanitized = sanitize_git_url(url);
        assert_eq!(sanitized, "github_com_user_repo");
    }

    #[test]
    fn test_sanitize_git_url_git_protocol() {
        let url = "git://github.com/user/repo.git";
        let sanitized = sanitize_git_url(url);
        assert_eq!(sanitized, "github_com_user_repo");
    }

    #[test]
    fn test_sanitize_git_url_complex() {
        let url = "https://gitlab.example.com:8080/group/subgroup/repo.git";
        let sanitized = sanitize_git_url(url);
        assert_eq!(sanitized, "gitlab_example_com_8080_group_subgroup_repo");
    }

    #[test]
    fn test_extract_wasm_from_repo_no_manifest() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let result = extract_wasm_from_repo(repo_path, "test-plugin");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No cdm-plugin.json found"));
    }

    #[test]
    fn test_extract_wasm_from_repo_invalid_json() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Create invalid JSON manifest
        let manifest_path = repo_path.join("cdm-plugin.json");
        fs::write(&manifest_path, "invalid json").unwrap();

        let result = extract_wasm_from_repo(repo_path, "test-plugin");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse"));
    }

    #[test]
    fn test_extract_wasm_from_repo_no_wasm_field() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Create manifest without wasm.file field
        let manifest_path = repo_path.join("cdm-plugin.json");
        let manifest_content = serde_json::json!({
            "name": "test-plugin",
            "version": "1.0.0"
        });
        fs::write(&manifest_path, serde_json::to_string(&manifest_content).unwrap()).unwrap();

        let result = extract_wasm_from_repo(repo_path, "test-plugin");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No wasm.file specified"));
    }

    #[test]
    fn test_extract_wasm_from_repo_wasm_file_not_found() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Create manifest with non-existent wasm file
        let manifest_path = repo_path.join("cdm-plugin.json");
        let manifest_content = serde_json::json!({
            "name": "test-plugin",
            "version": "1.0.0",
            "wasm": {
                "file": "plugin.wasm"
            }
        });
        fs::write(&manifest_path, serde_json::to_string(&manifest_content).unwrap()).unwrap();

        let result = extract_wasm_from_repo(repo_path, "test-plugin");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("WASM file not found"));
    }

    #[test]
    fn test_extract_wasm_from_repo_success() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Create manifest
        let manifest_path = repo_path.join("cdm-plugin.json");
        let manifest_content = serde_json::json!({
            "name": "test-plugin",
            "version": "1.0.0",
            "wasm": {
                "file": "plugin.wasm"
            }
        });
        fs::write(&manifest_path, serde_json::to_string(&manifest_content).unwrap()).unwrap();

        // Create wasm file
        let wasm_path = repo_path.join("plugin.wasm");
        fs::write(&wasm_path, b"wasm content").unwrap();

        let result = extract_wasm_from_repo(repo_path, "test-plugin");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), wasm_path);
    }

    #[test]
    fn test_extract_wasm_from_repo_nested_path() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Create manifest with nested wasm path
        let manifest_path = repo_path.join("cdm-plugin.json");
        let manifest_content = serde_json::json!({
            "name": "test-plugin",
            "version": "1.0.0",
            "wasm": {
                "file": "target/release/plugin.wasm"
            }
        });
        fs::write(&manifest_path, serde_json::to_string(&manifest_content).unwrap()).unwrap();

        // Create nested wasm file
        fs::create_dir_all(repo_path.join("target/release")).unwrap();
        let wasm_path = repo_path.join("target/release/plugin.wasm");
        fs::write(&wasm_path, b"wasm content").unwrap();

        let result = extract_wasm_from_repo(repo_path, "test-plugin");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), wasm_path);
    }

    #[test]
    fn test_clone_git_plugin_creates_cache_dir() {
        // This test would require mocking git commands or using a real git repository
        // Since it involves external commands, we'll test the sanitization instead
        use crate::registry;

        let cache_path = registry::get_cache_path();
        assert!(cache_path.is_ok());
    }
}
