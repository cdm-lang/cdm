use crate::self_update::error::UpdateError;

/// Get the current platform triple for binary downloads
pub fn get_current_platform() -> Result<String, UpdateError> {
    let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        (os, arch) => {
            return Err(UpdateError::UnsupportedPlatform(format!("{}-{}", os, arch)));
        }
    };

    Ok(platform.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        // This test will pass on supported platforms
        let result = get_current_platform();
        assert!(result.is_ok());

        let platform = result.unwrap();
        assert!(
            platform == "x86_64-apple-darwin"
                || platform == "aarch64-apple-darwin"
                || platform == "x86_64-unknown-linux-gnu"
                || platform == "aarch64-unknown-linux-gnu"
                || platform == "x86_64-pc-windows-msvc"
        );
    }
}
