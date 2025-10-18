/// Configuration management for .env files
///
/// Handles reading, writing, and validating IGRA Orchestra configuration

use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::{generate_hex_string, is_valid_domain, is_valid_email, is_valid_hex, RPC_TOKEN_COUNT};

#[derive(Debug, Clone)]
pub struct ConfigValue {
    pub key: String,
    pub value: String,
    pub comment: Option<String>,
}

pub struct ConfigManager {
    env_file: PathBuf,
    config: HashMap<String, ConfigValue>,
}

impl ConfigManager {
    /// Load configuration from .env file
    pub fn load<P: AsRef<Path>>(env_file: P) -> Result<Self> {
        let env_file = env_file.as_ref().to_path_buf();

        if !env_file.exists() {
            return Err(anyhow!(".env file not found at {}", env_file.display()));
        }

        let content = fs::read_to_string(&env_file)
            .context("Failed to read .env file")?;

        let mut config = HashMap::new();
        let mut current_comment = None;

        for line in content.lines() {
            let line = line.trim();

            // Handle comments
            if line.starts_with('#') {
                current_comment = Some(line.trim_start_matches('#').trim().to_string());
                continue;
            }

            // Skip empty lines
            if line.is_empty() {
                current_comment = None;
                continue;
            }

            // Parse key=value
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();

                config.insert(
                    key.clone(),
                    ConfigValue {
                        key: key.clone(),
                        value,
                        comment: current_comment.take(),
                    },
                );
            }
        }

        Ok(Self { env_file, config })
    }

    /// Save configuration to .env file
    pub fn save(&self) -> Result<()> {
        let mut lines = Vec::new();

        // Preserve order by reading original file
        let original = fs::read_to_string(&self.env_file)?;
        for line in original.lines() {
            let line_trimmed = line.trim();

            if line_trimmed.starts_with('#') || line_trimmed.is_empty() {
                lines.push(line.to_string());
            } else if let Some((key, _)) = line_trimmed.split_once('=') {
                let key = key.trim();
                if let Some(value) = self.config.get(key) {
                    lines.push(format!("{}={}", key, value.value));
                } else {
                    lines.push(line.to_string());
                }
            }
        }

        fs::write(&self.env_file, lines.join("\n"))
            .context("Failed to write .env file")?;

        Ok(())
    }

    /// Get a configuration value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.config.get(key).map(|v| v.value.as_str())
    }

    /// Set a configuration value
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();

        if let Some(existing) = self.config.get_mut(&key) {
            existing.value = value;
        } else {
            self.config.insert(
                key.clone(),
                ConfigValue {
                    key: key.clone(),
                    value,
                    comment: None,
                },
            );
        }
    }

    /// Get all RPC access tokens
    pub fn get_rpc_tokens(&self) -> Vec<(usize, Option<String>)> {
        (1..=RPC_TOKEN_COUNT)
            .map(|i| {
                let key = format!("RPC_ACCESS_TOKEN_{}", i);
                let token = self.get(&key).map(|s| s.to_string());
                (i, token)
            })
            .collect()
    }

    /// Generate a single RPC access token
    pub fn generate_rpc_token(&mut self, index: usize) -> Result<String> {
        if index < 1 || index > RPC_TOKEN_COUNT {
            return Err(anyhow!(
                "Token index must be between 1 and {}",
                RPC_TOKEN_COUNT
            ));
        }

        let token = generate_hex_string(32);
        let key = format!("RPC_ACCESS_TOKEN_{}", index);
        self.set(key, &token);

        Ok(token)
    }

    /// Generate all RPC access tokens
    pub fn generate_all_rpc_tokens(&mut self) -> Result<Vec<String>> {
        let mut tokens = Vec::new();

        for i in 1..=RPC_TOKEN_COUNT {
            let token = self.generate_rpc_token(i)?;
            tokens.push(token);
        }

        Ok(tokens)
    }

    /// Get wallet configuration for a specific worker
    pub fn get_wallet_config(&self, worker_id: usize) -> Option<WalletConfig> {
        let address_key = format!("W{}_WALLET_TO_ADDRESS", worker_id);
        let password_key = format!("W{}_KASWALLET_PASSWORD", worker_id);

        let address = self.get(&address_key)?;

        Some(WalletConfig {
            worker_id,
            address: address.to_string(),
            password: self.get(&password_key).map(|s| s.to_string()),
        })
    }

    /// Get domain configuration
    pub fn get_domain_config(&self) -> Option<DomainConfig> {
        let domain = self.get("IGRA_ORCHESTRA_DOMAIN")?;
        let email = self.get("IGRA_ORCHESTRA_DOMAIN_EMAIL")?;

        Some(DomainConfig {
            domain: domain.to_string(),
            email: email.to_string(),
            ovh_endpoint: self.get("OVH_ENDPOINT").map(|s| s.to_string()),
            ovh_app_key: self.get("OVH_APPLICATION_KEY").map(|s| s.to_string()),
            ovh_app_secret: self.get("OVH_APPLICATION_SECRET").map(|s| s.to_string()),
            ovh_consumer_key: self.get("OVH_CONSUMER_KEY").map(|s| s.to_string()),
        })
    }

    /// Validate configuration
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // Check required fields
        if self.get("NETWORK").is_none() {
            errors.push("NETWORK is not set".to_string());
        }

        if self.get("NODE_ID").is_none() {
            errors.push("NODE_ID is not set".to_string());
        }

        // Validate domain if set
        if let Some(domain) = self.get("IGRA_ORCHESTRA_DOMAIN") {
            if !is_valid_domain(domain) {
                errors.push(format!("Invalid domain: {}", domain));
            }
        }

        // Validate email if set
        if let Some(email) = self.get("IGRA_ORCHESTRA_DOMAIN_EMAIL") {
            if !is_valid_email(email) {
                errors.push(format!("Invalid email: {}", email));
            }
        }

        // Check RPC tokens (at least token 1 should be set)
        if self.get("RPC_ACCESS_TOKEN_1").is_none() {
            errors.push("No RPC access tokens configured".to_string());
        }

        // Validate existing tokens are hex
        for (i, token) in self.get_rpc_tokens() {
            if let Some(t) = token {
                if !is_valid_hex(&t) || t.len() != 32 {
                    errors.push(format!(
                        "RPC_ACCESS_TOKEN_{} must be 32 hex characters",
                        i
                    ));
                }
            }
        }

        errors
    }

    /// Get all configuration keys
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.config.keys().cloned().collect();
        keys.sort();
        keys
    }

    /// Export configuration to HashMap
    pub fn to_map(&self) -> HashMap<String, String> {
        self.config
            .iter()
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct WalletConfig {
    pub worker_id: usize,
    pub address: String,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DomainConfig {
    pub domain: String,
    pub email: String,
    pub ovh_endpoint: Option<String>,
    pub ovh_app_key: Option<String>,
    pub ovh_app_secret: Option<String>,
    pub ovh_consumer_key: Option<String>,
}

impl DomainConfig {
    pub fn has_ovh_config(&self) -> bool {
        self.ovh_endpoint.is_some()
            && self.ovh_app_key.is_some()
            && self.ovh_app_secret.is_some()
            && self.ovh_consumer_key.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_manager() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "# Test configuration").unwrap();
        writeln!(file, "NETWORK=testnet").unwrap();
        writeln!(file, "NODE_ID=test-node").unwrap();
        writeln!(file, "RPC_ACCESS_TOKEN_1=deadbeefdeadbeefdeadbeefdeadbeef").unwrap();

        let config = ConfigManager::load(file.path()).unwrap();

        assert_eq!(config.get("NETWORK"), Some("testnet"));
        assert_eq!(config.get("NODE_ID"), Some("test-node"));

        let errors = config.validate();
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_generate_tokens() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "NETWORK=testnet").unwrap();

        let mut config = ConfigManager::load(file.path()).unwrap();

        let token = config.generate_rpc_token(1).unwrap();
        assert_eq!(token.len(), 32);
        assert!(is_valid_hex(&token));
    }
}
