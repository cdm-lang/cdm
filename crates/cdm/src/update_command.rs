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

    #[test]
    fn test_update_commands_structure() {
        // This test just verifies the command structure compiles
        // Actual functionality is tested in integration tests
    }
}
