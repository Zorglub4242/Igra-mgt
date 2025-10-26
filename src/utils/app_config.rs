/// Application configuration management
/// Stores user preferences in ~/.config/igra-cli/config.toml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Represents a detected project with its compose configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectConfig {
    pub name: String,              // "igra" or "kasplex" or "other"
    pub compose_file: String,      // path to docker-compose.yml
    pub working_dir: String,       // docker compose project directory
    pub env_file: Option<String>,  // optional .env file
}

impl ProjectConfig {
    pub fn new(
        name: impl Into<String>,
        compose_file: impl Into<String>,
        working_dir: impl Into<String>,
        env_file: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            compose_file: compose_file.into(),
            working_dir: working_dir.into(),
            env_file,
        }
    }

    pub fn compose_file_path(&self) -> PathBuf {
        PathBuf::from(&self.compose_file)
    }

    pub fn working_dir_path(&self) -> PathBuf {
        PathBuf::from(&self.working_dir)
    }

    pub fn env_file_path(&self) -> Option<PathBuf> {
        self.env_file.as_ref().map(PathBuf::from)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub project_root: Option<String>,
    #[serde(default)]
    pub show_all_containers: bool,  // Show containers from all projects
    #[serde(default)]
    pub projects: Vec<ProjectConfig>,  // Detected projects
}

impl AppConfig {
    /// Get config file path
    pub fn config_path() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .context("HOME environment variable not set")?;
        let config_dir = PathBuf::from(home).join(".config").join("igra-cli");

        // Create directory if it doesn't exist
        fs::create_dir_all(&config_dir)
            .context("Failed to create config directory")?;

        Ok(config_dir.join("config.toml"))
    }

    /// Load configuration from file
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(Self {
                project_root: None,
                show_all_containers: false,
                projects: Vec::new(),
            });
        }

        let contents = fs::read_to_string(&path)
            .context("Failed to read config file")?;

        let config: Self = toml::from_str(&contents)
            .context("Failed to parse config file")?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        fs::write(&path, contents)
            .context("Failed to write config file")?;

        Ok(())
    }

    /// Set and save project root
    pub fn set_project_root(&mut self, root: PathBuf) -> Result<()> {
        self.project_root = Some(root.to_string_lossy().to_string());
        self.save()
    }
}
