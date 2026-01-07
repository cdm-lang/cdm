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

    #[test]
    fn test_platform_string_format() {
        let result = get_current_platform();
        assert!(result.is_ok());

        let platform = result.unwrap();
        // Platform strings should contain hyphens
        assert!(platform.contains("-"));

        // Platform strings should have at least 2 parts
        let parts: Vec<&str> = platform.split('-').collect();
        assert!(parts.len() >= 2);
    }

    #[test]
    fn test_platform_not_empty() {
        let result = get_current_platform();
        assert!(result.is_ok());

        let platform = result.unwrap();
        assert!(!platform.is_empty());
    }

    #[test]
    fn test_platform_is_valid_rust_target() {
        let result = get_current_platform();
        assert!(result.is_ok());

        let platform = result.unwrap();

        // All our supported platforms are valid Rust target triples
        // They should start with an architecture
        let valid_archs = ["x86_64", "aarch64"];
        let starts_with_valid_arch = valid_archs.iter().any(|arch| platform.starts_with(arch));
        assert!(starts_with_valid_arch);
    }

    #[test]
    fn test_platform_deterministic() {
        // Multiple calls should return the same value
        let result1 = get_current_platform();
        let result2 = get_current_platform();

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_platform_macos() {
        let result = get_current_platform();
        assert!(result.is_ok());

        let platform = result.unwrap();
        assert!(platform.contains("apple-darwin"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_platform_linux() {
        let result = get_current_platform();
        assert!(result.is_ok());

        let platform = result.unwrap();
        assert!(platform.contains("unknown-linux-gnu"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_platform_windows() {
        let result = get_current_platform();
        assert!(result.is_ok());

        let platform = result.unwrap();
        assert!(platform.contains("pc-windows-msvc"));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_platform_x86_64() {
        let result = get_current_platform();
        assert!(result.is_ok());

        let platform = result.unwrap();
        assert!(platform.starts_with("x86_64"));
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn test_platform_aarch64() {
        let result = get_current_platform();
        assert!(result.is_ok());

        let platform = result.unwrap();
        assert!(platform.starts_with("aarch64"));
    }
}
