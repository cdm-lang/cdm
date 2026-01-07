use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum UpdateCommands {
    /// Update to the latest version or a specific version
    #[command(name = "version")]
    Version {
        /// Specific version to install (e.g., "0.2.0")
        version: String,

        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Check if an update is available
    Check,
    /// List all available versions
    List,
}

/// Handle update command without subcommand (update to latest)
pub fn update_latest(yes: bool) -> Result<()> {
    use crate::self_update;

    match self_update::update_to_version(None, yes) {
        Ok(()) => Ok(()),
        Err(self_update::UpdateError::AlreadyLatest(version)) => {
            println!("Already on the latest version: {}", version);
            Ok(())
        }
        Err(e) => {
            anyhow::bail!("{}", e);
        }
    }
}

/// Handle update subcommands
pub fn handle_update_subcommand(command: UpdateCommands) -> Result<()> {
    use crate::self_update;

    match command {
        UpdateCommands::Version { version, yes } => {
            match self_update::update_to_version(Some(&version), yes) {
                Ok(()) => Ok(()),
                Err(self_update::UpdateError::AlreadyLatest(v)) => {
                    println!("Already on version: {}", v);
                    Ok(())
                }
                Err(e) => {
                    anyhow::bail!("{}", e);
                }
            }
        }
        UpdateCommands::Check => {
            match self_update::check_for_update() {
                Ok(Some(latest_version)) => {
                    let current_version = env!("CARGO_PKG_VERSION");
                    println!("Update available!");
                    println!("  Current version: {}", current_version);
                    println!("  Latest version:  {}", latest_version);
                    println!();
                    println!("Run 'cdm update' to update to the latest version");
                    Ok(())
                }
                Ok(None) => {
                    println!("You are on the latest version: {}", env!("CARGO_PKG_VERSION"));
                    Ok(())
                }
                Err(e) => {
                    anyhow::bail!("Failed to check for updates: {}", e);
                }
            }
        }
        UpdateCommands::List => {
            match self_update::list_versions() {
                Ok(versions) => {
                    let current_version = env!("CARGO_PKG_VERSION");
                    println!("Available versions:\n");
                    for version in versions {
                        let current_marker = if version == current_version {
                            " (current)"
                        } else {
                            ""
                        };
                        println!("  {}{}", version, current_marker);
                    }
                    println!();
                    println!("Use 'cdm update version <VERSION>' to install a specific version");
                    Ok(())
                }
                Err(e) => {
                    anyhow::bail!("Failed to list versions: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ENUM STRUCTURE TESTS
    // =========================================================================

    #[test]
    fn test_update_commands_structure() {
        // This test just verifies the command structure compiles
        // Actual functionality is tested in integration tests
    }

    #[test]
    fn test_update_commands_version_variant() {
        let cmd = UpdateCommands::Version {
            version: "1.2.3".to_string(),
            yes: false,
        };

        match cmd {
            UpdateCommands::Version { version, yes } => {
                assert_eq!(version, "1.2.3");
                assert!(!yes);
            }
            _ => panic!("Expected Version variant"),
        }
    }

    #[test]
    fn test_update_commands_version_with_yes() {
        let cmd = UpdateCommands::Version {
            version: "2.0.0".to_string(),
            yes: true,
        };

        match cmd {
            UpdateCommands::Version { version, yes } => {
                assert_eq!(version, "2.0.0");
                assert!(yes);
            }
            _ => panic!("Expected Version variant"),
        }
    }

    #[test]
    fn test_update_commands_check_variant() {
        let cmd = UpdateCommands::Check;

        match cmd {
            UpdateCommands::Check => (),
            _ => panic!("Expected Check variant"),
        }
    }

    #[test]
    fn test_update_commands_list_variant() {
        let cmd = UpdateCommands::List;

        match cmd {
            UpdateCommands::List => (),
            _ => panic!("Expected List variant"),
        }
    }

    // =========================================================================
    // VERSION STRING TESTS
    // =========================================================================

    #[test]
    fn test_version_string_formats() {
        // Test various version string formats that should be accepted
        let versions = vec![
            "0.1.0",
            "1.0.0",
            "1.2.3",
            "10.20.30",
            "0.0.1",
            "1.0.0-alpha",
            "1.0.0-beta.1",
            "1.0.0-rc.1",
        ];

        for version in versions {
            let cmd = UpdateCommands::Version {
                version: version.to_string(),
                yes: false,
            };

            if let UpdateCommands::Version { version: v, .. } = cmd {
                assert_eq!(v, version);
            }
        }
    }

    #[test]
    fn test_current_version_available() {
        // Verify we can access the current version
        let current = env!("CARGO_PKG_VERSION");
        assert!(!current.is_empty());

        // Should be a valid semver
        let parts: Vec<&str> = current.split('.').collect();
        assert!(parts.len() >= 2); // At least major.minor
    }

    // =========================================================================
    // COMMAND PATTERN TESTS
    // =========================================================================

    #[test]
    fn test_update_commands_pattern_matching() {
        // Test that all variants can be pattern matched
        let commands = vec![
            UpdateCommands::Version {
                version: "1.0.0".to_string(),
                yes: false,
            },
            UpdateCommands::Check,
            UpdateCommands::List,
        ];

        for cmd in commands {
            match cmd {
                UpdateCommands::Version { .. } => (),
                UpdateCommands::Check => (),
                UpdateCommands::List => (),
            }
        }
    }

    #[test]
    fn test_version_empty_string() {
        // Empty version string should still be constructible
        // (validation happens at runtime in update_to_version)
        let cmd = UpdateCommands::Version {
            version: String::new(),
            yes: true,
        };

        if let UpdateCommands::Version { version, .. } = cmd {
            assert!(version.is_empty());
        }
    }
}
