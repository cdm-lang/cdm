use crate::self_update::error::UpdateError;
use std::path::{Path, PathBuf};
use std::fs;

/// Replace the current binary with a new one
pub fn replace_current_binary(new_binary_path: &Path) -> Result<(), UpdateError> {
    let current_exe = std::env::current_exe()
        .map_err(|e| UpdateError::IoError(e))?;

    // Create backup of current binary
    let backup_path = get_backup_path(&current_exe)?;

    if current_exe.exists() {
        fs::copy(&current_exe, &backup_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    UpdateError::PermissionDenied(e)
                } else {
                    UpdateError::IoError(e)
                }
            })?;
    }

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(new_binary_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(new_binary_path, perms)?;
    }

    // Replace the binary
    #[cfg(unix)]
    {
        replace_unix(&current_exe, new_binary_path, &backup_path)?;
    }

    #[cfg(windows)]
    {
        replace_windows(&current_exe, new_binary_path, &backup_path)?;
    }

    Ok(())
}

#[cfg(unix)]
fn replace_unix(current_exe: &Path, new_binary: &Path, backup_path: &Path) -> Result<(), UpdateError> {
    // On Unix, we can atomically replace the file
    fs::copy(new_binary, current_exe)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                // Try to restore backup
                let _ = fs::copy(backup_path, current_exe);
                UpdateError::PermissionDenied(e)
            } else {
                // Try to restore backup
                let _ = fs::copy(backup_path, current_exe);
                UpdateError::IoError(e)
            }
        })?;

    // Clean up temporary file
    let _ = fs::remove_file(new_binary);

    Ok(())
}

#[cfg(windows)]
fn replace_windows(current_exe: &Path, new_binary: &Path, _backup_path: &Path) -> Result<(), UpdateError> {
    // On Windows, we can't replace a running executable directly
    // Instead, we'll copy the new binary next to the current one with a .new extension
    // and instruct the user to restart

    let new_exe_path = current_exe.with_extension("exe.new");

    fs::copy(new_binary, &new_exe_path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                UpdateError::PermissionDenied(e)
            } else {
                UpdateError::IoError(e)
            }
        })?;

    // Create a batch script to replace the binary on next run
    let script_path = current_exe.with_extension("update.bat");
    let script_content = format!(
        "@echo off\n\
         timeout /t 2 /nobreak > nul\n\
         move /y \"{}\" \"{}\"\n\
         del \"%~f0\"\n",
        new_exe_path.display(),
        current_exe.display()
    );

    fs::write(&script_path, script_content)?;

    println!("\nNote: On Windows, the update requires a restart.");
    println!("Please close this terminal and run 'cdm' again to complete the update.");

    Ok(())
}

fn get_backup_path(current_exe: &Path) -> Result<PathBuf, UpdateError> {
    let backup_name = format!(
        "{}.backup",
        current_exe.file_name()
            .ok_or_else(|| UpdateError::IoError(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid executable path")
            ))?
            .to_string_lossy()
    );

    Ok(current_exe.with_file_name(backup_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_backup_path() {
        let exe_path = PathBuf::from("/usr/local/bin/cdm");
        let backup = get_backup_path(&exe_path).unwrap();
        assert_eq!(backup, PathBuf::from("/usr/local/bin/cdm.backup"));
    }

    #[test]
    #[cfg(windows)]
    fn test_get_backup_path_windows() {
        let exe_path = PathBuf::from("C:\\Program Files\\cdm\\cdm.exe");
        let backup = get_backup_path(&exe_path).unwrap();
        assert_eq!(backup, PathBuf::from("C:\\Program Files\\cdm\\cdm.exe.backup"));
    }
}
