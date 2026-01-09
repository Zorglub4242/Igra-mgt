/// Project detection logic
/// Detects IGRA, Kasplex, and other Docker Compose projects
use anyhow::{Context, Result};
use bollard::container::ListContainersOptions;
use bollard::Docker;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::utils::app_config::ProjectConfig;

/// Detect all available Docker Compose projects
pub async fn detect_projects() -> Result<Vec<ProjectConfig>> {
    let mut projects = Vec::new();

    // 1. Try to detect IGRA Orchestra project
    if let Some(igra_project) = detect_igra_project().await? {
        projects.push(igra_project);
    }

    // 2. Try to detect Kasplex project
    if let Some(kasplex_project) = detect_kasplex_project().await? {
        projects.push(kasplex_project);
    }

    // 3. Detect other running compose projects from Docker labels
    let other_projects = detect_other_compose_projects().await?;
    projects.extend(other_projects);

    Ok(projects)
}

/// Detect IGRA Orchestra project
async fn detect_igra_project() -> Result<Option<ProjectConfig>> {
    // Try to find IGRA project root
    let project_root = match crate::utils::get_project_root() {
        Ok(root) => root,
        Err(_) => return Ok(None),
    };

    let compose_file = project_root.join("docker-compose.yml");
    if !compose_file.exists() {
        return Ok(None);
    }

    // Read compose file and check if it contains viaduct service (IGRA signature)
    let compose_content = tokio::fs::read_to_string(&compose_file)
        .await
        .context("Failed to read docker-compose.yml")?;

    if !compose_content.contains("viaduct") {
        return Ok(None); // Not an IGRA project
    }

    let env_file = project_root.join(".env");
    let env_file_str = if env_file.exists() {
        Some(env_file.to_string_lossy().to_string())
    } else {
        None
    };

    Ok(Some(ProjectConfig::new(
        "igra",
        compose_file.to_string_lossy().to_string(),
        project_root.to_string_lossy().to_string(),
        env_file_str,
    )))
}

/// Detect Kasplex project
async fn detect_kasplex_project() -> Result<Option<ProjectConfig>> {
    // Known Kasplex location
    let home_dir = dirs::home_dir()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from));

    let home_dir = match home_dir {
        Some(dir) => dir,
        None => return Ok(None),
    };

    let kasplex_root = home_dir.join("kasplex").join("evm-l2-nodes-docker");

    if !kasplex_root.exists() {
        return Ok(None);
    }

    let compose_file = kasplex_root.join("docker-compose").join("syncer.yml");
    if !compose_file.exists() {
        return Ok(None);
    }

    let working_dir = kasplex_root.join("docker");
    if !working_dir.exists() {
        return Ok(None);
    }

    // Check for environment files
    let testnet_env = kasplex_root
        .join("docker-compose")
        .join("envs")
        .join("syncer.testnet.env");
    let mainnet_env = kasplex_root
        .join("docker-compose")
        .join("envs")
        .join("syncer.mainnet.env");

    // Default to testnet env
    let env_file = if testnet_env.exists() {
        Some(testnet_env.to_string_lossy().to_string())
    } else if mainnet_env.exists() {
        Some(mainnet_env.to_string_lossy().to_string())
    } else {
        None
    };

    Ok(Some(ProjectConfig::new(
        "kasplex",
        compose_file.to_string_lossy().to_string(),
        working_dir.to_string_lossy().to_string(),
        env_file,
    )))
}

/// Detect other Docker Compose projects from running containers
async fn detect_other_compose_projects() -> Result<Vec<ProjectConfig>> {
    let docker =
        Docker::connect_with_local_defaults().context("Failed to connect to Docker daemon")?;

    let mut filters = HashMap::new();
    filters.insert("label", vec!["com.docker.compose.project"]);

    let options = ListContainersOptions {
        all: true,
        filters,
        ..Default::default()
    };

    let containers = docker
        .list_containers(Some(options))
        .await
        .context("Failed to list containers")?;

    let mut projects_map: HashMap<String, (String, String, Option<String>)> = HashMap::new();

    for container in containers {
        if let Some(labels) = container.labels {
            let project_name = labels.get("com.docker.compose.project");
            let config_files = labels.get("com.docker.compose.project.config_files");
            let working_dir = labels.get("com.docker.compose.project.working_dir");
            let env_file = labels.get("com.docker.compose.project.environment_file");

            if let (Some(name), Some(config), Some(workdir)) =
                (project_name, config_files, working_dir)
            {
                // Skip IGRA and Kasplex as they're detected separately
                if name == "igra-orchestra-testnet"
                    || name == "igra-orchestra-mainnet"
                    || name == "docker"
                {
                    continue;
                }

                projects_map.insert(
                    name.clone(),
                    (config.clone(), workdir.clone(), env_file.cloned()),
                );
            }
        }
    }

    Ok(projects_map
        .into_iter()
        .map(|(name, (compose_file, working_dir, env_file))| {
            ProjectConfig::new(name, compose_file, working_dir, env_file)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_projects() {
        let _ = detect_projects().await;
    }
}
