/// Docker image version checking
///
/// Queries Docker Hub and GitHub to check for latest versions

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ImageVersion {
    pub current: String,
    pub latest: Option<String>,
    pub update_available: bool,
}

#[derive(Deserialize)]
struct DockerHubResponse {
    results: Vec<DockerHubTag>,
}

#[derive(Deserialize)]
struct DockerHubTag {
    name: String,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Check latest version from Docker Hub
async fn get_docker_hub_latest(image_name: &str) -> Result<String> {
    let url = format!("https://hub.docker.com/v2/repositories/{}/tags?page_size=100", image_name);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let response: DockerHubResponse = client
        .get(&url)
        .send()
        .await?
        .json()
        .await?;

    // Find the latest semantic version tag (ignore 'latest', 'main', etc.)
    let latest = response.results
        .iter()
        .find(|tag| tag.name.starts_with('v') && tag.name.chars().nth(1).map(|c| c.is_numeric()).unwrap_or(false))
        .map(|tag| tag.name.clone())
        .unwrap_or_else(|| "latest".to_string());

    Ok(latest)
}

/// Check latest version from GitHub releases
async fn get_github_latest(repo: &str) -> Result<String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent("igra-cli")
        .build()?;

    let response: GitHubRelease = client
        .get(&url)
        .send()
        .await?
        .json()
        .await?;

    Ok(response.tag_name)
}

/// Check versions for all known IGRA images
pub async fn check_versions(current_images: HashMap<String, String>) -> HashMap<String, ImageVersion> {
    let mut versions = HashMap::new();

    for (image_full, current_tag) in current_images {
        // Extract just the image name
        let image_name = image_full
            .split('/')
            .last()
            .unwrap_or(&image_full)
            .split(':')
            .next()
            .unwrap_or(&image_full);

        // Map to Docker Hub repo or GitHub
        let latest = match image_name {
            "kaspad" | "rusty-kaspa" => {
                // Check GitHub releases for kaspanet/rusty-kaspa
                get_github_latest("kaspanet/rusty-kaspa").await.ok()
            }
            "execution-layer" | "reth" => {
                // Check GitHub releases for paradigmxyz/reth
                get_github_latest("paradigmxyz/reth").await.ok()
            }
            "block-builder" => {
                // IgraLabs image - check Docker Hub
                get_docker_hub_latest("igranetwork/block-builder").await.ok()
            }
            "viaduct" => {
                get_docker_hub_latest("igranetwork/viaduct").await.ok()
            }
            "rpc-provider" => {
                get_docker_hub_latest("igranetwork/rpc-provider").await.ok()
            }
            "kaswallet" => {
                get_docker_hub_latest("igranetwork/kaswallet").await.ok()
            }
            _ => None,
        };

        let update_available = if let Some(ref latest_tag) = latest {
            &current_tag != "latest" && &current_tag != latest_tag
        } else {
            false
        };

        versions.insert(
            image_full,
            ImageVersion {
                current: current_tag,
                latest,
                update_available,
            },
        );
    }

    versions
}
