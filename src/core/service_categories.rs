/// Service Category Management
///
/// Manages user-defined categories for organizing system services
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCategory {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub services: Vec<String>,
    pub order: i32,
    pub is_default: bool,
    pub is_active: bool, // For filtering like Docker profiles
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryConfig {
    pub categories: Vec<ServiceCategory>,
    pub tracked_services: HashMap<String, TrackedService>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedService {
    pub category: String,
    pub display_name: String,
    pub metrics_enabled: bool,
    pub plugin: Option<String>,
}

pub struct CategoryManager {
    config_path: PathBuf,
    config: CategoryConfig,
}

impl CategoryManager {
    pub fn new(config_dir: PathBuf) -> Result<Self> {
        let config_path = config_dir.join("service_categories.json");

        let config = if config_path.exists() {
            let content =
                fs::read_to_string(&config_path).context("Failed to read category config")?;
            serde_json::from_str(&content).context("Failed to parse category config")?
        } else {
            Self::default_config()
        };

        Ok(Self {
            config_path,
            config,
        })
    }

    /// Get default category configuration
    fn default_config() -> CategoryConfig {
        CategoryConfig {
            categories: vec![
                ServiceCategory {
                    id: "system".to_string(),
                    name: "System".to_string(),
                    icon: "⚙️".to_string(),
                    color: "#64748b".to_string(),
                    services: vec![
                        "systemd-journald.service".to_string(),
                        "systemd-logind.service".to_string(),
                        "systemd-udevd.service".to_string(),
                        "dbus.service".to_string(),
                    ],
                    order: 1,
                    is_default: true,
                    is_active: true,
                },
                ServiceCategory {
                    id: "networking".to_string(),
                    name: "Networking".to_string(),
                    icon: "🌐".to_string(),
                    color: "#3b82f6".to_string(),
                    services: vec![
                        "networking.service".to_string(),
                        "NetworkManager.service".to_string(),
                        "systemd-networkd.service".to_string(),
                        "systemd-resolved.service".to_string(),
                    ],
                    order: 2,
                    is_default: true,
                    is_active: true,
                },
                ServiceCategory {
                    id: "file-sharing".to_string(),
                    name: "File Sharing".to_string(),
                    icon: "📁".to_string(),
                    color: "#f59e0b".to_string(),
                    services: vec![
                        "smbd.service".to_string(),
                        "nmbd.service".to_string(),
                        "nfs-server.service".to_string(),
                        "rpcbind.service".to_string(),
                    ],
                    order: 3,
                    is_default: true,
                    is_active: true,
                },
                ServiceCategory {
                    id: "node-services".to_string(),
                    name: "Node Services".to_string(),
                    icon: "🔗".to_string(),
                    color: "#6366f1".to_string(),
                    services: vec![
                        "kaspa-mainnet.service".to_string(),
                        "kaspa-resolver.service".to_string(),
                        "kaspa-testnet.service".to_string(),
                    ],
                    order: 4,
                    is_default: true,
                    is_active: true,
                },
                ServiceCategory {
                    id: "web-services".to_string(),
                    name: "Web Services".to_string(),
                    icon: "🌍".to_string(),
                    color: "#10b981".to_string(),
                    services: vec![
                        "nginx.service".to_string(),
                        "apache2.service".to_string(),
                        "caddy.service".to_string(),
                    ],
                    order: 5,
                    is_default: true,
                    is_active: true,
                },
                ServiceCategory {
                    id: "databases".to_string(),
                    name: "Databases".to_string(),
                    icon: "💾".to_string(),
                    color: "#ec4899".to_string(),
                    services: vec![
                        "postgresql.service".to_string(),
                        "mysql.service".to_string(),
                        "mariadb.service".to_string(),
                        "mongodb.service".to_string(),
                        "redis.service".to_string(),
                    ],
                    order: 6,
                    is_default: true,
                    is_active: true,
                },
                ServiceCategory {
                    id: "game-servers".to_string(),
                    name: "Game Servers".to_string(),
                    icon: "🎮".to_string(),
                    color: "#8b5cf6".to_string(),
                    services: vec![
                        "minecraft.service".to_string(),
                        "valheim.service".to_string(),
                        "terraria.service".to_string(),
                    ],
                    order: 7,
                    is_default: true,
                    is_active: true,
                },
            ],
            tracked_services: HashMap::new(),
        }
    }

    /// Get all categories
    pub fn get_categories(&self) -> Vec<ServiceCategory> {
        let mut cats = self.config.categories.clone();
        cats.sort_by_key(|c| c.order);
        cats
    }

    /// Get a specific category by ID
    pub fn get_category(&self, id: &str) -> Option<ServiceCategory> {
        self.config.categories.iter().find(|c| c.id == id).cloned()
    }

    /// Add a new category
    pub fn add_category(&mut self, category: ServiceCategory) -> Result<()> {
        // Check if ID already exists
        if self.config.categories.iter().any(|c| c.id == category.id) {
            return Err(anyhow::anyhow!(
                "Category with ID '{}' already exists",
                category.id
            ));
        }

        self.config.categories.push(category);
        self.save_config()
    }

    /// Update an existing category
    pub fn update_category(&mut self, id: &str, updated: ServiceCategory) -> Result<()> {
        let index = self
            .config
            .categories
            .iter()
            .position(|c| c.id == id)
            .ok_or_else(|| anyhow::anyhow!("Category '{}' not found", id))?;

        self.config.categories[index] = updated;
        self.save_config()
    }

    /// Delete a category
    pub fn delete_category(&mut self, id: &str) -> Result<()> {
        // Don't allow deletion of default categories
        let category = self
            .get_category(id)
            .ok_or_else(|| anyhow::anyhow!("Category '{}' not found", id))?;

        if category.is_default {
            return Err(anyhow::anyhow!("Cannot delete default category"));
        }

        self.config.categories.retain(|c| c.id != id);

        // Remove category from tracked services
        for tracked in self.config.tracked_services.values_mut() {
            if tracked.category == id {
                tracked.category = "Uncategorized".to_string();
            }
        }

        self.save_config()
    }

    /// Add a service to a category
    pub fn add_service_to_category(
        &mut self,
        category_id: &str,
        service_name: String,
    ) -> Result<()> {
        let category = self
            .config
            .categories
            .iter_mut()
            .find(|c| c.id == category_id)
            .ok_or_else(|| anyhow::anyhow!("Category '{}' not found", category_id))?;

        if !category.services.contains(&service_name) {
            category.services.push(service_name.clone());
        }

        // Update tracked service
        self.config
            .tracked_services
            .entry(service_name.clone())
            .and_modify(|t| t.category = category_id.to_string())
            .or_insert(TrackedService {
                category: category_id.to_string(),
                display_name: service_name.trim_end_matches(".service").to_string(),
                metrics_enabled: true,
                plugin: None,
            });

        self.save_config()
    }

    /// Remove a service from a category
    pub fn remove_service_from_category(
        &mut self,
        category_id: &str,
        service_name: &str,
    ) -> Result<()> {
        let category = self
            .config
            .categories
            .iter_mut()
            .find(|c| c.id == category_id)
            .ok_or_else(|| anyhow::anyhow!("Category '{}' not found", category_id))?;

        category.services.retain(|s| s != service_name);
        self.save_config()
    }

    /// Get category for a service
    pub fn get_service_category(&self, service_name: &str) -> String {
        if let Some(tracked) = self.config.tracked_services.get(service_name) {
            return tracked.category.clone();
        }

        // Check default categories
        for category in &self.config.categories {
            if category.services.contains(&service_name.to_string()) {
                return category.id.clone();
            }
        }

        "Uncategorized".to_string()
    }

    /// Get tracked services configuration
    pub fn get_tracked_services(&self) -> &HashMap<String, TrackedService> {
        &self.config.tracked_services
    }

    /// Add or update tracked service
    pub fn update_tracked_service(&mut self, name: String, tracked: TrackedService) -> Result<()> {
        self.config.tracked_services.insert(name, tracked);
        self.save_config()
    }

    /// Remove tracked service
    pub fn remove_tracked_service(&mut self, name: &str) -> Result<()> {
        self.config.tracked_services.remove(name);

        // Remove from all categories
        for category in &mut self.config.categories {
            category.services.retain(|s| s != name);
        }

        self.save_config()
    }

    /// Check if a service is tracked
    pub fn is_tracked(&self, service_name: &str) -> bool {
        self.config.tracked_services.contains_key(service_name)
            || self
                .config
                .categories
                .iter()
                .any(|c| c.services.contains(&service_name.to_string()))
    }

    /// Reorder categories
    pub fn reorder_categories(&mut self, category_ids: Vec<String>) -> Result<()> {
        for (index, id) in category_ids.iter().enumerate() {
            if let Some(category) = self.config.categories.iter_mut().find(|c| &c.id == id) {
                category.order = index as i32;
            }
        }

        self.save_config()
    }

    /// Save configuration to disk
    fn save_config(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.config)
            .context("Failed to serialize category config")?;

        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        fs::write(&self.config_path, content).context("Failed to write category config")?;

        Ok(())
    }

    /// Export categories configuration
    pub fn export_config(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.config).context("Failed to export config")
    }

    /// Import categories configuration
    pub fn import_config(&mut self, json: &str) -> Result<()> {
        let new_config: CategoryConfig =
            serde_json::from_str(json).context("Failed to parse imported config")?;

        self.config = new_config;
        self.save_config()
    }
}
