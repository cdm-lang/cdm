use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::registry;

/// Clone or update a git plugin repository and return the path to the WASM file
pub fn clone_git_plugin(url: &str, git_ref: &str) -> Result<PathBuf> {
    clone_git_plugin_with_cache_path(url, git_ref, &registry::get_cache_path()?)
}

/// Clone or update a git plugin repository with explicit cache path (for testing)
pub(crate) fn clone_git_plugin_with_cache_path(url: &str, git_ref: &str, cache_path: &Path) -> Result<PathBuf> {
    let cache_dir = cache_path.join("git");
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
///
/// If `subdir` is provided, looks for cdm-plugin.json in that subdirectory.
/// Otherwise, looks in the repository root.
pub fn extract_wasm_from_repo(repo_path: &Path, subdir: Option<&str>) -> Result<PathBuf> {
    // Determine the base path (repo root or subdirectory)
    let base_path = if let Some(sub) = subdir {
        repo_path.join(sub)
    } else {
        repo_path.to_path_buf()
    };

    // Read cdm-plugin.json manifest
    let manifest_path = base_path.join("cdm-plugin.json");
    if !manifest_path.exists() {
        if let Some(sub) = subdir {
            anyhow::bail!(
                "No cdm-plugin.json found in repository subdirectory: {}\nFull path: {}",
                sub,
                manifest_path.display()
            );
        } else {
            anyhow::bail!(
                "No cdm-plugin.json found in repository at {}",
                repo_path.display()
            );
        }
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

    let wasm_path = base_path.join(wasm_file);
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
        .env("GIT_TERMINAL_PROMPT", "0")
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
        .env("GIT_TERMINAL_PROMPT", "0")
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
        .env("GIT_TERMINAL_PROMPT", "0")
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
        .env("GIT_TERMINAL_PROMPT", "0")
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
#[path = "git_plugin/git_plugin_tests.rs"]
mod git_plugin_tests;
