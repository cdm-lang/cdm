use std::fmt;

#[derive(Debug)]
pub enum UpdateError {
    NetworkError(reqwest::Error),
    InvalidManifest(String),
    UnsupportedPlatform(String),
    ChecksumMismatch { expected: String, actual: String },
    PermissionDenied(std::io::Error),
    VersionNotFound(String),
    AlreadyLatest(String),
    IoError(std::io::Error),
    JsonError(serde_json::Error),
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateError::NetworkError(e) => write!(f, "Network error: {}", e),
            UpdateError::InvalidManifest(msg) => write!(f, "Invalid manifest: {}", msg),
            UpdateError::UnsupportedPlatform(platform) => {
                write!(f, "Platform '{}' is not supported. Pre-built binaries are not available for this platform.", platform)
            }
            UpdateError::ChecksumMismatch { expected, actual } => {
                write!(f, "Checksum mismatch!\n  Expected: {}\n  Actual:   {}", expected, actual)
            }
            UpdateError::PermissionDenied(e) => {
                write!(f, "Permission denied: {}. Try running with sudo or check file permissions.", e)
            }
            UpdateError::VersionNotFound(version) => {
                write!(f, "Version '{}' not found in release manifest", version)
            }
            UpdateError::AlreadyLatest(version) => {
                write!(f, "Already on version {}", version)
            }
            UpdateError::IoError(e) => write!(f, "I/O error: {}", e),
            UpdateError::JsonError(e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl std::error::Error for UpdateError {}

impl From<reqwest::Error> for UpdateError {
    fn from(err: reqwest::Error) -> Self {
        UpdateError::NetworkError(err)
    }
}

impl From<std::io::Error> for UpdateError {
    fn from(err: std::io::Error) -> Self {
        UpdateError::IoError(err)
    }
}

impl From<serde_json::Error> for UpdateError {
    fn from(err: serde_json::Error) -> Self {
        UpdateError::JsonError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // DISPLAY TESTS
    // =========================================================================

    #[test]
    fn test_display_invalid_manifest() {
        let err = UpdateError::InvalidManifest("bad json".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Invalid manifest"));
        assert!(display.contains("bad json"));
    }

    #[test]
    fn test_display_unsupported_platform() {
        let err = UpdateError::UnsupportedPlatform("riscv64-unknown-linux".to_string());
        let display = format!("{}", err);
        assert!(display.contains("riscv64-unknown-linux"));
        assert!(display.contains("not supported"));
    }

    #[test]
    fn test_display_checksum_mismatch() {
        let err = UpdateError::ChecksumMismatch {
            expected: "sha256:abc123".to_string(),
            actual: "sha256:def456".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Checksum mismatch"));
        assert!(display.contains("sha256:abc123"));
        assert!(display.contains("sha256:def456"));
    }

    #[test]
    fn test_display_permission_denied() {
        let io_err = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "access denied"
        );
        let err = UpdateError::PermissionDenied(io_err);
        let display = format!("{}", err);
        assert!(display.contains("Permission denied"));
        assert!(display.contains("sudo") || display.contains("permissions"));
    }

    #[test]
    fn test_display_version_not_found() {
        let err = UpdateError::VersionNotFound("999.0.0".to_string());
        let display = format!("{}", err);
        assert!(display.contains("999.0.0"));
        assert!(display.contains("not found"));
    }

    #[test]
    fn test_display_already_latest() {
        let err = UpdateError::AlreadyLatest("1.2.3".to_string());
        let display = format!("{}", err);
        assert!(display.contains("1.2.3"));
        assert!(display.contains("Already"));
    }

    #[test]
    fn test_display_io_error() {
        let io_err = std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found"
        );
        let err = UpdateError::IoError(io_err);
        let display = format!("{}", err);
        assert!(display.contains("I/O error"));
    }

    #[test]
    fn test_display_json_error() {
        let json_str = "{ invalid json }";
        let json_err: Result<serde_json::Value, _> = serde_json::from_str(json_str);
        let err = UpdateError::JsonError(json_err.unwrap_err());
        let display = format!("{}", err);
        assert!(display.contains("JSON error"));
    }

    // =========================================================================
    // DEBUG TESTS
    // =========================================================================

    #[test]
    fn test_debug_invalid_manifest() {
        let err = UpdateError::InvalidManifest("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("InvalidManifest"));
    }

    #[test]
    fn test_debug_checksum_mismatch() {
        let err = UpdateError::ChecksumMismatch {
            expected: "a".to_string(),
            actual: "b".to_string(),
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("ChecksumMismatch"));
        assert!(debug.contains("expected"));
        assert!(debug.contains("actual"));
    }

    #[test]
    fn test_debug_version_not_found() {
        let err = UpdateError::VersionNotFound("1.0.0".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("VersionNotFound"));
    }

    #[test]
    fn test_debug_already_latest() {
        let err = UpdateError::AlreadyLatest("2.0.0".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("AlreadyLatest"));
    }

    #[test]
    fn test_debug_unsupported_platform() {
        let err = UpdateError::UnsupportedPlatform("unknown".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("UnsupportedPlatform"));
    }

    // =========================================================================
    // FROM TRAIT TESTS
    // =========================================================================

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "test error"
        );
        let err: UpdateError = io_err.into();
        match err {
            UpdateError::IoError(_) => (),
            _ => panic!("Expected IoError variant"),
        }
    }

    #[test]
    fn test_from_json_error() {
        let json_err: Result<serde_json::Value, _> = serde_json::from_str("invalid");
        let err: UpdateError = json_err.unwrap_err().into();
        match err {
            UpdateError::JsonError(_) => (),
            _ => panic!("Expected JsonError variant"),
        }
    }

    // =========================================================================
    // ERROR TRAIT TESTS
    // =========================================================================

    #[test]
    fn test_error_trait_implemented() {
        let err = UpdateError::InvalidManifest("test".to_string());
        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_error_trait_display() {
        let err = UpdateError::InvalidManifest("test".to_string());
        let error_ref: &dyn std::error::Error = &err;
        let display = format!("{}", error_ref);
        assert!(display.contains("Invalid manifest"));
    }

    // =========================================================================
    // EDGE CASES
    // =========================================================================

    #[test]
    fn test_empty_string_variants() {
        let err1 = UpdateError::InvalidManifest(String::new());
        let err2 = UpdateError::VersionNotFound(String::new());
        let err3 = UpdateError::AlreadyLatest(String::new());
        let err4 = UpdateError::UnsupportedPlatform(String::new());

        // All should format without panic
        let _ = format!("{}", err1);
        let _ = format!("{}", err2);
        let _ = format!("{}", err3);
        let _ = format!("{}", err4);
    }

    #[test]
    fn test_special_characters_in_messages() {
        let err = UpdateError::InvalidManifest("error with \"quotes\" and 'apostrophes'".to_string());
        let display = format!("{}", err);
        assert!(display.contains("quotes"));
    }

    #[test]
    fn test_unicode_in_messages() {
        let err = UpdateError::InvalidManifest("错误消息 🚫".to_string());
        let display = format!("{}", err);
        assert!(display.contains("错误消息"));
    }
}
