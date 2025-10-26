/// Service notes storage and default notes provider
///
/// Provides user-editable notes for services with sensible defaults based on container image patterns.
/// Notes are stored in JSON format in the user's config directory.

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;

/// Service notes storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceNotes {
    /// Custom notes set by the user
    #[serde(default)]
    custom_notes: HashMap<String, String>,
}

impl ServiceNotes {
    /// Load service notes from config file
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read service notes from {}", path.display()))?;

        let notes: ServiceNotes = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse service notes from {}", path.display()))?;

        Ok(notes)
    }

    /// Save service notes to config file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize service notes")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write service notes to {}", path.display()))?;

        Ok(())
    }

    /// Get note for a service (returns default if not customized)
    pub fn get_note(&self, service_name: &str, container_image: &str) -> String {
        // Check for custom note first
        if let Some(custom) = self.custom_notes.get(service_name) {
            return custom.clone();
        }

        // Return default note based on image pattern
        Self::default_note_for_image(container_image)
    }

    /// Set a custom note for a service
    pub fn set_note(&mut self, service_name: String, note: String) {
        if note.is_empty() {
            // Remove custom note if empty (will fall back to default)
            self.custom_notes.remove(&service_name);
        } else {
            self.custom_notes.insert(service_name, note);
        }
    }

    /// Reset a service to its default note
    pub fn reset_to_default(&mut self, service_name: &str) {
        self.custom_notes.remove(service_name);
    }

    /// Get the config file path
    fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Failed to determine config directory")?;

        Ok(config_dir.join("igra-cli").join("service_notes.json"))
    }

    /// Get default note based on container image
    fn default_note_for_image(image: &str) -> String {
        let image_lower = image.to_lowercase();

        // Match image patterns to default descriptions
        if image_lower.contains("reth") || image_lower.contains("execution-layer") {
            "Reth Ethereum execution client. Provides EVM compatibility and execution environment for KASPA L2. \
             Exposes JSON-RPC API for transaction submission and state queries.".to_string()
        } else if image_lower.contains("geth") {
            "Geth (Go Ethereum) execution client. Provides EVM compatibility and execution environment. \
             Alternative to Reth for running the execution layer.".to_string()
        } else if image_lower.contains("kaspad") {
            "Kaspa L1 node. Provides base layer security, consensus, and fast block confirmations. \
             Required for entry transactions and L1 finality.".to_string()
        } else if image_lower.contains("viaduct") || image_lower.contains("bridge") {
            "Viaduct L1→L2 bridge. Monitors Kaspa for entry transactions, processes them after confirmation, \
             and forwards to the block builder. Maintains sync state in RocksDB.".to_string()
        } else if image_lower.contains("block-builder") || image_lower.contains("blockbuilder") {
            "Block builder service. Receives L1 data from Viaduct, constructs L2 blocks with entry transactions, \
             and submits them to the execution layer via Engine API.".to_string()
        } else if image_lower.contains("traefik") {
            "Traefik reverse proxy and load balancer. Handles SSL/TLS termination, token-based routing, \
             and distributes requests across RPC provider workers.".to_string()
        } else if image_lower.contains("rpc-provider") || image_lower.contains("rpcprovider") {
            "RPC provider worker. Proxies Ethereum JSON-RPC requests to execution layer, handles entry transactions \
             via Kaswallet for KAS→iKAS bridging. Can scale horizontally (up to 5 workers).".to_string()
        } else if image_lower.contains("kaswallet") {
            "Kaspa wallet daemon. Signs and submits entry transactions for the RPC provider. \
             One wallet instance required per RPC worker.".to_string()
        } else if image_lower.contains("kaspa-miner") {
            "Kaspa CPU miner. Optional service for isolated development environments. \
             Mines blocks on local Kaspa testnet.".to_string()
        } else if image_lower.contains("postgres") || image_lower.contains("postgresql") {
            "PostgreSQL database. Stores persistent data for services that require SQL storage.".to_string()
        } else if image_lower.contains("redis") {
            "Redis in-memory data store. Provides caching and pub/sub functionality.".to_string()
        } else if image_lower.contains("nginx") {
            "Nginx web server and reverse proxy. Serves static content and routes HTTP traffic.".to_string()
        } else {
            format!("Service running {} image.", image)
        }
    }
}

impl Default for ServiceNotes {
    fn default() -> Self {
        Self {
            custom_notes: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_notes() {
        let notes = ServiceNotes::default();

        // Test reth default note
        let reth_note = notes.get_note("execution-layer", "reth:latest");
        assert!(reth_note.contains("Reth"));
        assert!(reth_note.contains("EVM"));

        // Test kaspad default note
        let kaspad_note = notes.get_note("kaspad", "kaspad:v1.0.0");
        assert!(kaspad_note.contains("Kaspa L1"));

        // Test unknown image
        let unknown_note = notes.get_note("custom", "myimage:latest");
        assert!(unknown_note.contains("myimage:latest"));
    }

    #[test]
    fn test_custom_notes() {
        let mut notes = ServiceNotes::default();

        // Set custom note
        notes.set_note("execution-layer".to_string(), "My custom note".to_string());

        // Custom note should override default
        let note = notes.get_note("execution-layer", "reth:latest");
        assert_eq!(note, "My custom note");

        // Reset to default
        notes.reset_to_default("execution-layer");
        let note = notes.get_note("execution-layer", "reth:latest");
        assert!(note.contains("Reth"));
    }

    #[test]
    fn test_empty_note_removes_custom() {
        let mut notes = ServiceNotes::default();

        // Set custom note
        notes.set_note("test".to_string(), "Custom".to_string());
        assert_eq!(notes.get_note("test", "test:latest"), "Custom");

        // Empty note should remove custom
        notes.set_note("test".to_string(), "".to_string());
        let note = notes.get_note("test", "test:latest");
        assert!(note.contains("test:latest")); // Should return default
    }
}
