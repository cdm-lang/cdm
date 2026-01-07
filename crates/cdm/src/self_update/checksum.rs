use crate::self_update::error::UpdateError;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Verify checksum of a file
pub fn verify_file(path: &Path, expected_checksum: &str) -> Result<(), UpdateError> {
    let data = fs::read(path)?;
    verify_bytes(&data, expected_checksum)
}

/// Verify checksum of byte data
pub fn verify_bytes(data: &[u8], expected_checksum: &str) -> Result<(), UpdateError> {
    // Parse expected checksum format: "sha256:hexstring"
    let parts: Vec<&str> = expected_checksum.split(':').collect();
    if parts.len() != 2 {
        return Err(UpdateError::InvalidManifest(format!(
            "Invalid checksum format: {}",
            expected_checksum
        )));
    }

    let (algorithm, expected_hash) = (parts[0], parts[1]);

    match algorithm {
        "sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(data);
            let actual_hash = format!("{:x}", hasher.finalize());

            if actual_hash != expected_hash {
                return Err(UpdateError::ChecksumMismatch {
                    expected: format!("sha256:{}", expected_hash),
                    actual: format!("sha256:{}", actual_hash),
                });
            }
        }
        _ => {
            return Err(UpdateError::InvalidManifest(format!(
                "Unsupported checksum algorithm: {}",
                algorithm
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // =========================================================================
    // verify_bytes TESTS
    // =========================================================================

    #[test]
    fn test_verify_bytes_valid() {
        let data = b"test data";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:x}", hasher.finalize());
        let checksum = format!("sha256:{}", hash);

        assert!(verify_bytes(data, &checksum).is_ok());
    }

    #[test]
    fn test_verify_bytes_invalid() {
        let data = b"test data";
        let checksum = "sha256:invalid";

        let result = verify_bytes(data, checksum);
        assert!(result.is_err());
        match result.unwrap_err() {
            UpdateError::ChecksumMismatch { expected, actual } => {
                assert_eq!(expected, "sha256:invalid");
                assert!(actual.starts_with("sha256:"));
            }
            _ => panic!("Expected ChecksumMismatch error"),
        }
    }

    #[test]
    fn test_verify_bytes_bad_format() {
        let data = b"test data";
        let checksum = "invalid_format";

        let result = verify_bytes(data, checksum);
        assert!(result.is_err());
        match result.unwrap_err() {
            UpdateError::InvalidManifest(msg) => {
                assert!(msg.contains("Invalid checksum format"));
            }
            _ => panic!("Expected InvalidManifest error"),
        }
    }

    #[test]
    fn test_verify_bytes_unsupported_algorithm() {
        let data = b"test data";
        let checksum = "md5:d8e8fca2dc0f896fd7cb4cb0031ba249";

        let result = verify_bytes(data, checksum);
        assert!(result.is_err());
        match result.unwrap_err() {
            UpdateError::InvalidManifest(msg) => {
                assert!(msg.contains("Unsupported checksum algorithm"));
                assert!(msg.contains("md5"));
            }
            _ => panic!("Expected InvalidManifest error"),
        }
    }

    #[test]
    fn test_verify_bytes_empty_data() {
        let data = b"";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:x}", hasher.finalize());
        let checksum = format!("sha256:{}", hash);

        assert!(verify_bytes(data, &checksum).is_ok());
    }

    #[test]
    fn test_verify_bytes_large_data() {
        // Test with 1MB of data
        let data = vec![0u8; 1024 * 1024];
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash = format!("{:x}", hasher.finalize());
        let checksum = format!("sha256:{}", hash);

        assert!(verify_bytes(&data, &checksum).is_ok());
    }

    #[test]
    fn test_verify_bytes_binary_data() {
        // Test with binary data including null bytes
        let data: Vec<u8> = (0..=255).collect();
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash = format!("{:x}", hasher.finalize());
        let checksum = format!("sha256:{}", hash);

        assert!(verify_bytes(&data, &checksum).is_ok());
    }

    #[test]
    fn test_verify_bytes_uppercase_hash() {
        // Checksums should be lowercase, uppercase should fail
        let data = b"test data";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:X}", hasher.finalize()); // uppercase
        let checksum = format!("sha256:{}", hash);

        // Should fail because our implementation uses lowercase
        assert!(verify_bytes(data, &checksum).is_err());
    }

    #[test]
    fn test_verify_bytes_extra_colons() {
        let data = b"test data";
        let checksum = "sha256:abc:def";

        // Should fail due to invalid format (more than one colon)
        let result = verify_bytes(data, checksum);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_bytes_empty_algorithm() {
        let data = b"test data";
        let checksum = ":abc123";

        let result = verify_bytes(data, checksum);
        assert!(result.is_err());
        match result.unwrap_err() {
            UpdateError::InvalidManifest(msg) => {
                assert!(msg.contains("Unsupported checksum algorithm"));
            }
            _ => panic!("Expected InvalidManifest error"),
        }
    }

    #[test]
    fn test_verify_bytes_empty_hash() {
        let data = b"test data";
        let checksum = "sha256:";

        let result = verify_bytes(data, checksum);
        assert!(result.is_err());
        match result.unwrap_err() {
            UpdateError::ChecksumMismatch { expected, .. } => {
                assert_eq!(expected, "sha256:");
            }
            _ => panic!("Expected ChecksumMismatch error"),
        }
    }

    // =========================================================================
    // verify_file TESTS
    // =========================================================================

    #[test]
    fn test_verify_file_valid() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file");

        let data = b"file content for testing";
        fs::write(&file_path, data).unwrap();

        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:x}", hasher.finalize());
        let checksum = format!("sha256:{}", hash);

        assert!(verify_file(&file_path, &checksum).is_ok());
    }

    #[test]
    fn test_verify_file_invalid_checksum() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file");

        fs::write(&file_path, b"file content").unwrap();

        let result = verify_file(&file_path, "sha256:wrong_checksum");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_file_not_found() {
        let result = verify_file(
            Path::new("/nonexistent/path/to/file"),
            "sha256:abc123"
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_file_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty_file");

        fs::write(&file_path, b"").unwrap();

        let mut hasher = Sha256::new();
        hasher.update(b"");
        let hash = format!("{:x}", hasher.finalize());
        let checksum = format!("sha256:{}", hash);

        assert!(verify_file(&file_path, &checksum).is_ok());
    }

    // =========================================================================
    // KNOWN VALUE TESTS
    // =========================================================================

    #[test]
    fn test_known_sha256_value() {
        // Known SHA256 hash for "hello world"
        let data = b"hello world";
        let known_hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        let checksum = format!("sha256:{}", known_hash);

        assert!(verify_bytes(data, &checksum).is_ok());
    }

    #[test]
    fn test_known_sha256_empty() {
        // Known SHA256 hash for empty string
        let data = b"";
        let known_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let checksum = format!("sha256:{}", known_hash);

        assert!(verify_bytes(data, &checksum).is_ok());
    }
}
