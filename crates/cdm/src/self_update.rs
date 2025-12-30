// Self-update functionality for the CDM CLI

mod checksum;
mod downloader;
mod error;
mod manifest;
mod platform;
mod replacer;

pub use error::UpdateError;

/// Check if an update is available
pub fn check_for_update() -> Result<Option<String>, UpdateError> {
    let current_version = env!("CARGO_PKG_VERSION");
    let manifest = manifest::fetch_manifest()?;

    if manifest.latest != current_version {
        Ok(Some(manifest.latest.clone()))
    } else {
        Ok(None)
    }
}

/// List all available versions
pub fn list_versions() -> Result<Vec<String>, UpdateError> {
    let manifest = manifest::fetch_manifest()?;
    let mut versions: Vec<String> = manifest.releases.keys().cloned().collect();

    // Try to sort by semver
    versions.sort_by(|a, b| {
        match (semver::Version::parse(a), semver::Version::parse(b)) {
            (Ok(va), Ok(vb)) => vb.cmp(&va), // Reverse order (newest first)
            _ => b.cmp(a),
        }
    });

    Ok(versions)
}

/// Update to a specific version (or latest if None)
pub fn update_to_version(version: Option<&str>, skip_confirm: bool) -> Result<(), UpdateError> {
    let current_version = env!("CARGO_PKG_VERSION");
    let manifest = manifest::fetch_manifest()?;

    // Determine target version
    let target_version = version.unwrap_or(&manifest.latest);

    // Check if version exists
    let release = manifest.releases.get(target_version)
        .ok_or_else(|| UpdateError::VersionNotFound(target_version.to_string()))?;

    // Check if already on target version
    if target_version == current_version {
        return Err(UpdateError::AlreadyLatest(current_version.to_string()));
    }

    // Get current platform
    let platform = platform::get_current_platform()?;

    // Get platform-specific release
    let platform_release = release.platforms.get(&platform)
        .ok_or_else(|| UpdateError::UnsupportedPlatform(platform.clone()))?;

    // Confirm with user unless --yes flag is set
    if !skip_confirm {
        println!("Current version: {}", current_version);
        println!("Target version:  {}", target_version);
        println!("Platform:        {}", platform);
        println!();
        print!("Continue with update? (y/N): ");

        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|e| UpdateError::IoError(e))?;

        if input.trim().to_lowercase() != "y" {
            println!("Update cancelled");
            return Ok(());
        }
    }

    // Download the new binary
    println!("Downloading CDM CLI v{}...", target_version);
    let temp_path = downloader::download_binary(&platform_release.url)?;

    // Verify checksum
    println!("Verifying checksum...");
    checksum::verify_file(&temp_path, &platform_release.checksum)?;

    // Replace the current binary
    println!("Installing...");
    replacer::replace_current_binary(&temp_path)?;

    println!("✓ Successfully updated to v{}", target_version);
    println!();
    println!("Note: The update will take effect when you restart the CLI");

    Ok(())
}
