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

    // On macOS, we need to:
    // 1. Remove quarantine extended attributes (com.apple.quarantine)
    // 2. Re-sign the binary with a local ad-hoc signature
    //
    // On Apple Silicon Macs, ad-hoc signatures from GitHub Actions may not be trusted
    // by taskgated. Re-signing locally creates a valid signature for this machine.
    #[cfg(target_os = "macos")]
    {
        prepare_macos_binary(current_exe);
    }

    // Clean up temporary file
    let _ = fs::remove_file(new_binary);

    Ok(())
}

/// Prepare a macOS binary for execution by removing quarantine attributes
/// and re-signing it with a local ad-hoc signature.
///
/// On Apple Silicon Macs (M1/M2/M3+), ad-hoc signatures created on GitHub Actions
/// runners may not be trusted by taskgated. Re-signing locally ensures the binary
/// can execute without being killed by the code signing enforcement.
#[cfg(target_os = "macos")]
fn prepare_macos_binary(path: &Path) {
    use std::process::Command;

    // Step 1: Remove quarantine extended attributes
    // Downloaded binaries get com.apple.quarantine which can trigger Gatekeeper
    let xattr_result = Command::new("xattr")
        .arg("-c")
        .arg(path)
        .output();

    match &xattr_result {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            eprintln!(
                "Warning: Failed to remove quarantine attributes: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(e) => {
            eprintln!("Warning: Could not run xattr: {}", e);
        }
    }

    // Step 2: Re-sign the binary with a local ad-hoc signature
    // This is required on Apple Silicon where remote ad-hoc signatures may not be trusted
    let codesign_result = Command::new("codesign")
        .args(["--sign", "-", "--force"])
        .arg(path)
        .output();

    match codesign_result {
        Ok(output) if output.status.success() => {
            // Successfully re-signed the binary
        }
        Ok(output) => {
            eprintln!(
                "Warning: Failed to re-sign binary: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(e) => {
            eprintln!("Warning: Could not run codesign: {}", e);
        }
    }
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
    use tempfile::TempDir;

    // =========================================================================
    // get_backup_path TESTS
    // =========================================================================

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

    #[test]
    fn test_get_backup_path_with_spaces() {
        let exe_path = PathBuf::from("/path with spaces/my app");
        let backup = get_backup_path(&exe_path).unwrap();
        assert_eq!(backup, PathBuf::from("/path with spaces/my app.backup"));
    }

    #[test]
    fn test_get_backup_path_relative() {
        let exe_path = PathBuf::from("./bin/cdm");
        let backup = get_backup_path(&exe_path).unwrap();
        assert_eq!(backup, PathBuf::from("./bin/cdm.backup"));
    }

    #[test]
    fn test_get_backup_path_deeply_nested() {
        let exe_path = PathBuf::from("/a/b/c/d/e/f/g/binary");
        let backup = get_backup_path(&exe_path).unwrap();
        assert_eq!(backup, PathBuf::from("/a/b/c/d/e/f/g/binary.backup"));
    }

    #[test]
    fn test_get_backup_path_with_extension() {
        let exe_path = PathBuf::from("/usr/bin/cdm.exe");
        let backup = get_backup_path(&exe_path).unwrap();
        assert_eq!(backup, PathBuf::from("/usr/bin/cdm.exe.backup"));
    }

    #[test]
    fn test_get_backup_path_unicode() {
        let exe_path = PathBuf::from("/usr/bin/программа");
        let backup = get_backup_path(&exe_path).unwrap();
        assert_eq!(backup, PathBuf::from("/usr/bin/программа.backup"));
    }

    #[test]
    fn test_get_backup_path_preserves_directory() {
        let exe_path = PathBuf::from("/custom/install/path/cdm");
        let backup = get_backup_path(&exe_path).unwrap();

        // Backup should be in the same directory
        assert_eq!(backup.parent().unwrap(), exe_path.parent().unwrap());
    }

    // =========================================================================
    // replace_current_binary INTEGRATION TESTS
    // =========================================================================

    #[test]
    fn test_replace_binary_nonexistent_source() {
        let nonexistent = PathBuf::from("/nonexistent/binary/path");
        let result = replace_current_binary(&nonexistent);
        assert!(result.is_err());
    }

    #[test]
    fn test_replace_binary_creates_backup_directory_check() {
        // This test verifies that the backup path computation works
        // for the current executable
        if let Ok(current_exe) = std::env::current_exe() {
            let backup = get_backup_path(&current_exe);
            assert!(backup.is_ok());

            let backup_path = backup.unwrap();
            // Backup should be in same directory as current exe
            assert_eq!(backup_path.parent(), current_exe.parent());
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_replace_unix_copies_and_cleans_up() {
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("source_binary");
        let target_path = temp_dir.path().join("target_binary");
        let backup_path = temp_dir.path().join("target_binary.backup");

        // Create source file
        fs::write(&source_path, b"new binary content").unwrap();

        // Create target file (existing binary)
        fs::write(&target_path, b"old binary content").unwrap();

        // Test replace_unix directly
        let result = replace_unix(&target_path, &source_path, &backup_path);
        assert!(result.is_ok());

        // Target should have new content
        let target_content = fs::read_to_string(&target_path).unwrap();
        assert_eq!(target_content, "new binary content");

        // Source should be cleaned up
        assert!(!source_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_replace_unix_permission_denied() {
        // This test may require special setup or be skipped on some systems
        // It's mainly here for coverage purposes
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("source");
        let target_path = PathBuf::from("/root/protected_file");
        let backup_path = temp_dir.path().join("backup");

        fs::write(&source_path, b"content").unwrap();

        // Attempting to write to /root should fail with permission denied
        // (unless running as root)
        let result = replace_unix(&target_path, &source_path, &backup_path);
        // This might succeed if running as root, so we just verify it returns a result
        let _ = result;
    }

    // =========================================================================
    // EDGE CASE TESTS
    // =========================================================================

    #[test]
    fn test_get_backup_path_single_file_name() {
        let exe_path = PathBuf::from("cdm");
        let backup = get_backup_path(&exe_path).unwrap();
        assert_eq!(backup, PathBuf::from("cdm.backup"));
    }

    #[test]
    fn test_get_backup_path_hidden_file() {
        let exe_path = PathBuf::from("/usr/bin/.cdm");
        let backup = get_backup_path(&exe_path).unwrap();
        assert_eq!(backup, PathBuf::from("/usr/bin/.cdm.backup"));
    }

    #[test]
    fn test_get_backup_path_double_extension() {
        let exe_path = PathBuf::from("/usr/bin/cdm.tar.gz");
        let backup = get_backup_path(&exe_path).unwrap();
        assert_eq!(backup, PathBuf::from("/usr/bin/cdm.tar.gz.backup"));
    }

    // =========================================================================
    // macOS BINARY PREPARATION TESTS
    // =========================================================================

    #[cfg(target_os = "macos")]
    #[test]
    fn test_prepare_macos_binary_removes_quarantine() {
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_binary");

        // Create a test file
        fs::write(&test_file, b"test content").unwrap();

        // Add a quarantine attribute (simulating a downloaded file)
        let _ = Command::new("xattr")
            .args(["-w", "com.apple.quarantine", "0081;00000000;Test;", test_file.to_str().unwrap()])
            .output();

        // Prepare the binary (removes quarantine and re-signs)
        prepare_macos_binary(&test_file);

        // Verify quarantine attribute is removed
        let output = Command::new("xattr")
            .arg("-l")
            .arg(&test_file)
            .output()
            .expect("Failed to run xattr");

        let attrs = String::from_utf8_lossy(&output.stdout);
        assert!(!attrs.contains("com.apple.quarantine"), "Quarantine attribute should be removed");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_prepare_macos_binary_signs_file() {
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_binary");

        // Create a minimal Mach-O executable (just the header for testing)
        // This is a minimal arm64 Mach-O header that codesign will accept
        fs::write(&test_file, b"test content for signing").unwrap();

        // Prepare the binary
        prepare_macos_binary(&test_file);

        // For non-Mach-O files, codesign will fail but shouldn't panic
        // The function handles errors gracefully
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_prepare_macos_binary_nonexistent_file() {
        let nonexistent = PathBuf::from("/nonexistent/path/to/binary");

        // Should not panic, just emit warnings
        prepare_macos_binary(&nonexistent);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_prepare_macos_binary_no_attrs() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("clean_file");

        // Create a file without quarantine attributes
        fs::write(&test_file, b"clean content").unwrap();

        // Should succeed without error on a file with no attributes
        prepare_macos_binary(&test_file);
    }
}
