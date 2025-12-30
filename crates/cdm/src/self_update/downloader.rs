use crate::self_update::error::UpdateError;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::io::Write;

/// Download a binary with progress indicator
pub fn download_binary(url: &str) -> Result<PathBuf, UpdateError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5 minutes for large binaries
        .build()?;

    let mut response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(UpdateError::NetworkError(
            reqwest::Error::from(response.error_for_status().unwrap_err())
        ));
    }

    // Get content length for progress bar
    let total_size = response.content_length().unwrap_or(0);

    // Create progress bar
    let pb = if total_size > 0 {
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .expect("Failed to set progress bar template")
                .progress_chars("#>-"),
        );
        Some(pb)
    } else {
        None
    };

    // Create temporary file
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path().join("cdm_update");
    let mut file = std::fs::File::create(&temp_path)?;

    // Download with progress
    let mut downloaded: u64 = 0;
    let mut buffer = vec![0; 8192]; // 8KB buffer

    loop {
        use std::io::Read;
        let bytes_read = response.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;

        if let Some(ref pb) = pb {
            pb.set_position(downloaded);
        }
    }

    if let Some(pb) = pb {
        pb.finish_with_message("Download complete");
    }

    // Prevent temp_dir from being dropped (which would delete the file)
    let path = temp_path.clone();
    std::mem::forget(temp_dir);

    Ok(path)
}
