use async_trait::async_trait;
use chrono::Utc;

use crate::{
    domain::{ScanAsset, ScanResult, ScanVerdict},
    ports::{ContentScanner, ScannerError},
};

const UNSAFE_EXTENSIONS: &[&str] = &[".exe", ".bat", ".cmd", ".dll", ".scr"];
const UNSAFE_NAME_MARKERS: &[&str] = &["malware", "eicar"];

#[derive(Clone, Copy, Debug, Default)]
pub struct SandboxContentScanner;

#[async_trait]
impl ContentScanner for SandboxContentScanner {
    async fn scan(&self, asset: ScanAsset) -> Result<ScanResult, ScannerError> {
        if asset.filename.trim().is_empty() || asset.size_bytes == 0 {
            return Err(ScannerError::InvalidAsset);
        }

        let filename = asset.filename.to_ascii_lowercase();
        let is_unsafe = UNSAFE_NAME_MARKERS
            .iter()
            .any(|marker| filename.contains(marker))
            || UNSAFE_EXTENSIONS.iter().any(|ext| filename.ends_with(ext));

        if is_unsafe {
            return Ok(ScanResult::new(
                &asset,
                ScanVerdict::Failed,
                "unsafe_file",
                Utc::now(),
            ));
        }

        Ok(ScanResult::new(
            &asset,
            ScanVerdict::Passed,
            "sandbox_clean",
            Utc::now(),
        ))
    }
}
