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

        assert!(verify_bytes(data, checksum).is_err());
    }

    #[test]
    fn test_verify_bytes_bad_format() {
        let data = b"test data";
        let checksum = "invalid_format";

        assert!(verify_bytes(data, checksum).is_err());
    }
}
