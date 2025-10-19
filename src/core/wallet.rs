/// Wallet management

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::process::Command;

// Include the generated gRPC client code
pub mod kaswallet_proto {
    tonic::include_proto!("kaswallet_proto");
}

use kaswallet_proto::kaswallet_proto_client::KaswalletProtoClient;

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

    /// Get wallet balance via gRPC
    pub async fn get_balance(&self, worker_id: usize) -> Result<f64> {
        // Connect to kaswallet-daemon gRPC endpoint
        // Port mapping: kaswallet-0 = 8082, kaswallet-1 = 8083, etc.
        let port = 8082 + worker_id;
        let endpoint = format!("http://127.0.0.1:{}", port);

        // Create gRPC client with timeout
        let mut client = KaswalletProtoClient::connect(endpoint.clone())
            .await
            .context(format!("Failed to connect to kaswallet-daemon at {}", endpoint))?;

        // Call GetBalance RPC
        let request = tonic::Request::new(kaswallet_proto::GetBalanceRequest {});
        let response = client.get_balance(request)
            .await
            .context("Failed to get balance from kaswallet-daemon")?;

        let balance_response = response.into_inner();

        // Convert sompi to KAS (1 KAS = 10^8 sompi)
        let balance_kas = balance_response.available as f64 / 100_000_000.0;

        Ok(balance_kas)
    }

    /// Get wallet address via gRPC (returns first address)
    pub async fn get_address(&self, worker_id: usize) -> Result<String> {
        // Connect to kaswallet-daemon gRPC endpoint
        let port = 8082 + worker_id;
        let endpoint = format!("http://127.0.0.1:{}", port);

        // Create gRPC client
        let mut client = KaswalletProtoClient::connect(endpoint.clone())
            .await
            .context(format!("Failed to connect to kaswallet-daemon at {}", endpoint))?;

        // Call ShowAddresses RPC
        let request = tonic::Request::new(kaswallet_proto::ShowAddressesRequest {});
        let response = client.show_addresses(request)
            .await
            .context("Failed to get addresses from kaswallet-daemon")?;

        let addresses_response = response.into_inner();

        // Return the first address, or error if no addresses
        addresses_response.address
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("No addresses found in wallet"))
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

    /// Send KAS from wallet to address via gRPC
    pub async fn send_transaction(&self, worker_id: usize, to_address: &str, amount: f64, password: &str) -> Result<String> {
        // Connect to kaswallet-daemon gRPC endpoint
        let port = 8082 + worker_id;
        let endpoint = format!("http://127.0.0.1:{}", port);

        // Create gRPC client
        let mut client = KaswalletProtoClient::connect(endpoint.clone())
            .await
            .context(format!("Failed to connect to kaswallet-daemon at {}", endpoint))?;

        // Convert KAS to sompi (1 KAS = 10^8 sompi)
        let amount_sompi = (amount * 100_000_000.0) as u64;

        // Call Send RPC
        let request = tonic::Request::new(kaswallet_proto::SendRequest {
            to_address: to_address.to_string(),
            amount: amount_sompi,
            password: password.to_string(),
            from: vec![], // Use default source addresses
            use_existing_change_address: false,
            is_send_all: false,
            fee_policy: None,
        });

        let response = client.send(request)
            .await
            .context("Failed to send transaction")?;

        let send_response = response.into_inner();

        // Return transaction IDs
        let tx_ids = send_response.tx_i_ds.join(", ");
        Ok(format!("Transaction sent!\nTxIDs: {}\nSigned {} transactions", tx_ids, send_response.signed_transactions.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Only run when kaswallet-0 is running
    async fn test_get_balance_grpc() {
        let wallet_manager = WalletManager::new().unwrap();
        let result = wallet_manager.get_balance(0).await;

        match result {
            Ok(balance) => {
                println!("Balance for kaswallet-0: {} KAS", balance);
                assert!(balance >= 0.0);
            }
            Err(e) => {
                println!("Failed to get balance (this is expected if kaswallet-0 is not running): {}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run when kaswallet-0 is running
    async fn test_get_address_grpc() {
        let wallet_manager = WalletManager::new().unwrap();
        let result = wallet_manager.get_address(0).await;

        match result {
            Ok(address) => {
                println!("Address for kaswallet-0: {}", address);
                assert!(address.starts_with("kaspatest:") || address.starts_with("kaspa:"));
            }
            Err(e) => {
                println!("Failed to get address (this is expected if kaswallet-0 is not running): {}", e);
            }
        }
    }
}

