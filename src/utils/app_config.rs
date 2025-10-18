/// Application configuration management
/// Stores user preferences in ~/.config/igra-cli/config.toml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub project_root: Option<String>,
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
