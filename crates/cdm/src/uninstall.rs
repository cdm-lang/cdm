use anyhow::{Result, Context};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use colored::Colorize;

/// Uninstall CDM CLI
pub fn uninstall(skip_confirmation: bool) -> Result<()> {
    // Determine installation directory
    let install_dir = if let Ok(custom_dir) = std::env::var("CDM_INSTALL_DIR") {
        PathBuf::from(custom_dir)
    } else {
        #[cfg(windows)]
        {
            let local_app_data = std::env::var("LOCALAPPDATA")
                .context("LOCALAPPDATA environment variable not found")?;
            PathBuf::from(local_app_data).join("cdm")
        }
        #[cfg(not(windows))]
        {
            let home = std::env::var("HOME")
                .context("HOME environment variable not found")?;
            PathBuf::from(home).join(".cdm")
        }
    };

    // Check if installation exists
    if !install_dir.exists() {
        println!("{} CDM CLI does not appear to be installed at {}",
            "Warning:".yellow(),
            install_dir.display()
        );
        println!("\n{} Checking for completions anyway...", "==>".green());
        remove_completions()?;
        return Ok(());
    }

    // Confirm uninstallation
    if !skip_confirmation {
        print!("\n{} This will uninstall CDM CLI from {}. Continue? [y/N] ",
            "Warning:".yellow(),
            install_dir.display()
        );
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        if !response.trim().eq_ignore_ascii_case("y") {
            println!("Uninstall cancelled.");
            return Ok(());
        }
    }

    println!("\n{} Uninstalling CDM CLI...", "==>".green());

    // Remove installation directory
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir)
            .context(format!("Failed to remove {}", install_dir.display()))?;
        println!("{} Removed CDM CLI from {}", "==>".green(), install_dir.display());
    }

    // Remove shell completions
    println!();
    remove_completions()?;

    // Remove plugin cache
    println!();
    remove_cache()?;

    println!("\n{} CDM CLI has been uninstalled successfully!", "==>".green());

    // Show manual cleanup instructions
    println!("\n{} You may want to:", "Note:".cyan());

    #[cfg(not(windows))]
    {
        println!("  - Remove this line from your shell profile (~/.bashrc, ~/.zshrc, etc.):");
        println!("    {}", format!("export PATH=\"$PATH:{}\"", install_dir.join("bin").display()).yellow());
        println!("  - Restart your shell: {}", "exec $SHELL".yellow());
    }

    #[cfg(windows)]
    {
        println!("  - Remove {} from your PATH environment variable", install_dir.join("bin").display());
        println!("  - Restart your terminal");
    }

    Ok(())
}

/// Remove plugin cache
fn remove_cache() -> Result<()> {
    // Determine cache directory
    let cache_dir = if let Ok(custom_cache) = std::env::var("CDM_CACHE_DIR") {
        PathBuf::from(custom_cache)
    } else {
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME")
                .context("HOME environment variable not found")?;
            PathBuf::from(home).join("Library/Caches/cdm")
        }
        #[cfg(target_os = "windows")]
        {
            let local_app_data = std::env::var("LOCALAPPDATA")
                .context("LOCALAPPDATA environment variable not found")?;
            PathBuf::from(local_app_data).join("cdm")
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let home = std::env::var("HOME")
                .context("HOME environment variable not found")?;
            let xdg_cache = std::env::var("XDG_CACHE_HOME")
                .unwrap_or_else(|_| format!("{}/.cache", home));
            PathBuf::from(xdg_cache).join("cdm")
        }
    };

    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)
            .context(format!("Failed to remove cache directory {}", cache_dir.display()))?;
        println!("{} Removed plugin cache from {}", "==>".green(), cache_dir.display());
    } else {
        println!("{} No plugin cache found", "==>".green());
    }

    Ok(())
}

/// Remove shell completions
fn remove_completions() -> Result<()> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .context("HOME/USERPROFILE environment variable not found")?;
    let home_path = PathBuf::from(home);

    let mut removed_count = 0;

    // Define all possible completion file locations
    let completion_files = vec![
        // Bash
        home_path.join(".local/share/bash-completion/completions/cdm"),
        home_path.join(".bash_completion.d/cdm"),
        // Zsh
        home_path.join(".zsh/completions/_cdm"),
        // Fish
        home_path.join(".config/fish/completions/cdm.fish"),
        // PowerShell (Windows)
        #[cfg(windows)]
        home_path.join("Documents/PowerShell/cdm-completion.ps1"),
    ];

    for completion_file in completion_files {
        if completion_file.exists() {
            fs::remove_file(&completion_file)
                .context(format!("Failed to remove {}", completion_file.display()))?;
            println!("{} Removed completions from {}", "==>".green(), completion_file.display());
            removed_count += 1;
        }
    }

    if removed_count == 0 {
        println!("{} No shell completions found", "==>".green());
    } else {
        println!();
        println!("{} You may want to remove completion setup from your shell config:", "Note:".cyan());

        #[cfg(not(windows))]
        {
            println!("  Zsh users: Remove these lines from ~/.zshrc:");
            println!("    {}", "fpath=(~/.zsh/completions $fpath)".yellow());
            println!("    {}", "autoload -Uz compinit && compinit".yellow());
            println!();
            println!("  Bash users: Remove this line from ~/.bashrc:");
            println!("    {}", "for f in ~/.bash_completion.d/*; do source $f; done".yellow());
        }

        #[cfg(windows)]
        {
            println!("  Remove this line from your PowerShell profile:");
            println!("    {}", ". \"$HOME\\Documents\\PowerShell\\cdm-completion.ps1\"".yellow());
        }
    }

    Ok(())
}
