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
