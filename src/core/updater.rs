/// Version checking and update management
/// Shared between TUI, Web UI API, and CLI

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use semver::Version;

const GITHUB_API_URL: &str = "https://api.github.com/repos/Zorglub4242/Igra-mgt/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO_URL: &str = "https://github.com/Zorglub4242/Igra-mgt";

/// Version information for display in TUI, Web UI, and CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_url: Option<String>,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
    pub download_url: Option<String>,
}

/// GitHub release API response
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    html_url: String,
    published_at: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Check for updates by querying GitHub releases API
/// This function is reused by TUI, Web API, and CLI
pub async fn check_for_updates() -> Result<VersionInfo> {
    // Parse current version
    let current = Version::parse(CURRENT_VERSION)
        .context("Failed to parse current version")?;

    // Fetch latest release from GitHub
    let client = reqwest::Client::builder()
        .user_agent(format!("igra-cli/{}", CURRENT_VERSION))
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client
        .get(GITHUB_API_URL)
        .send()
        .await
        .context("Failed to fetch latest release from GitHub")?;

    if !response.status().is_success() {
        // If we can't reach GitHub, return current version info without update check
        return Ok(VersionInfo {
            current_version: CURRENT_VERSION.to_string(),
            latest_version: None,
            update_available: false,
            release_url: Some(format!("{}/releases", REPO_URL)),
            release_notes: None,
            published_at: None,
            download_url: None,
        });
    }

    let release: GitHubRelease = response
        .json()
        .await
        .context("Failed to parse GitHub release response")?;

    // Parse latest version (remove 'v' prefix if present)
    let latest_tag = release.tag_name.trim_start_matches('v');
    let latest = Version::parse(latest_tag)
        .context("Failed to parse latest version")?;

    // Compare versions
    let update_available = latest > current;

    // Find Linux x86_64 binary asset
    let download_url = release.assets
        .iter()
        .find(|asset| asset.name.contains("linux-x86_64"))
        .map(|asset| asset.browser_download_url.clone());

    Ok(VersionInfo {
        current_version: CURRENT_VERSION.to_string(),
        latest_version: Some(latest.to_string()),
        update_available,
        release_url: Some(release.html_url),
        release_notes: release.body,
        published_at: Some(release.published_at),
        download_url,
    })
}

/// Get current version info without checking GitHub
pub fn get_current_version() -> VersionInfo {
    VersionInfo {
        current_version: CURRENT_VERSION.to_string(),
        latest_version: None,
        update_available: false,
        release_url: Some(format!("{}/releases", REPO_URL)),
        release_notes: None,
        published_at: None,
        download_url: None,
    }
}

/// Download latest release binary to specified path
pub async fn download_latest_release(destination: &std::path::Path) -> Result<()> {
    let version_info = check_for_updates().await?;

    let download_url = version_info.download_url
        .ok_or_else(|| anyhow::anyhow!("No download URL found for latest release"))?;

    let client = reqwest::Client::builder()
        .user_agent(format!("igra-cli/{}", CURRENT_VERSION))
        .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout for download
        .build()?;

    let response = client
        .get(&download_url)
        .send()
        .await
        .context("Failed to download release")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download release: HTTP {}", response.status());
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read release bytes")?;

    std::fs::write(destination, bytes)
        .context("Failed to write downloaded file")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_version_parsing() {
        let version = Version::parse(CURRENT_VERSION);
        assert!(version.is_ok(), "Current version should be valid semver");
    }

    #[test]
    fn test_get_current_version() {
        let info = get_current_version();
        assert_eq!(info.current_version, CURRENT_VERSION);
        assert!(!info.update_available);
    }
}
