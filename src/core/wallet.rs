/// Wallet management

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::process::Command;

pub struct WalletManager {
    project_root: std::path::PathBuf,
}

#[derive(Debug, Clone)]
pub struct WalletInfo {
    pub worker_id: usize,
    pub address: Option<String>,
    pub balance: Option<f64>,
    pub container_running: bool,
}

impl WalletManager {
    pub fn new() -> Result<Self> {
        let project_root = crate::utils::get_project_root()?;
        Ok(Self { project_root })
    }

    /// Get wallet balance
    /// TODO: Implement proper gRPC/HTTP API call to kaswallet-daemon
    pub async fn get_balance(&self, _worker_id: usize) -> Result<f64> {
        // For now, return an error to indicate balance is not available
        // The TUI will show "N/A" for balance
        // Future implementation should use the kaswallet-daemon gRPC API
        Err(anyhow!("Balance querying not yet implemented - requires gRPC API integration"))
    }

    /// Get wallet address from keys file
    pub async fn get_address(&self, worker_id: usize) -> Result<String> {
        let keys_file = self.project_root.join(format!("keys/keys.kaswallet-{}.json", worker_id));

        if !keys_file.exists() {
            return Err(anyhow!("Wallet keys file not found"));
        }

        let keys_content = std::fs::read_to_string(&keys_file)
            .context("Failed to read wallet keys file")?;

        let keys_json: Value = serde_json::from_str(&keys_content)
            .context("Failed to parse wallet keys file")?;

        // Get the public key
        if let Some(public_keys) = keys_json.get("public_keys").and_then(|pk| pk.as_array()) {
            if let Some(public_key) = public_keys.first().and_then(|pk| pk.as_str()) {
                // For now, just return the public key - in production you'd derive the address
                // The public key starts with "ktub" for testnet
                return Ok(format!("{}...", &public_key[..20]));
            }
        }

        Err(anyhow!("Failed to extract address from wallet keys"))
    }

    /// List all wallet information
    pub async fn list_wallets(&self) -> Result<Vec<WalletInfo>> {
        let mut wallets = Vec::new();

        for worker_id in 0..5 {
            let container_name = format!("kaswallet-{}", worker_id);

            // Check if container is running
            let container_running = Command::new("docker")
                .args(&["ps", "--filter", &format!("name={}", container_name), "--format", "{{.Names}}"])
                .output()
                .ok()
                .and_then(|output| {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Some(!stdout.trim().is_empty())
                })
                .unwrap_or(false);

            let (address, balance) = if container_running {
                let addr = self.get_address(worker_id).await.ok();
                let bal = self.get_balance(worker_id).await.ok();
                (addr, bal)
            } else {
                (None, None)
            };

            wallets.push(WalletInfo {
                worker_id,
                address,
                balance,
                container_running,
            });
        }

        Ok(wallets)
    }

    /// Generate a new wallet
    /// This checks if a keys file exists, if not returns an error
    pub async fn generate_wallet(&self, worker_id: usize, _password: &str) -> Result<String> {
        let keys_file = self.project_root.join(format!("keys/keys.kaswallet-{}.json", worker_id));

        if keys_file.exists() {
            // Wallet already exists, return its address
            return self.get_address(worker_id).await;
        }

        // For now, wallet generation needs to be done outside the TUI
        // by running kaswallet-create manually
        Err(anyhow!(
            "Wallet generation not yet implemented in TUI. \
             Please use: docker exec kaswallet-{} kaswallet-create --testnet --create",
            worker_id
        ))
    }

    /// Send KAS from wallet to address
    /// TODO: Implement proper gRPC/HTTP API call to kaswallet-daemon
    pub async fn send_transaction(&self, _worker_id: usize, _to_address: &str, _amount: f64, _password: &str) -> Result<String> {
        // For now, return an error indicating this feature requires gRPC implementation
        Err(anyhow!(
            "Transaction sending not yet implemented - requires gRPC API integration with kaswallet-daemon. \
             For now, please use the viaduct entry transaction endpoint or manual wallet commands."
        ))
    }
}
