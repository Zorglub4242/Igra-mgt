/// Wallet management

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

// Include the generated gRPC client code
pub mod kaswallet_proto {
    tonic::include_proto!("kaswallet_proto");
}

use kaswallet_proto::wallet_client::WalletClient;

pub struct WalletManager {
    project_root: std::path::PathBuf,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WalletInfo {
    pub worker_id: usize,
    pub address: Option<String>,
    pub balance: Option<f64>,
    pub container_running: bool,
    pub initial_balance: Option<f64>,
    pub fees_spent: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UtxoInfo {
    pub address: String,
    pub tx_id: String,
    pub amount_kas: f64,
    pub block_daa_score: u64,
    pub is_coinbase: bool,
    pub timestamp_ms: u64,  // Estimated timestamp in milliseconds
    pub source_addresses: Vec<String>,  // Source addresses for the transaction (empty for coinbase)
}

/// Persistent storage for wallet initial balances
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct WalletTracking {
    /// Map of worker_id -> initial_balance
    initial_balances: HashMap<usize, f64>,
}

impl WalletTracking {
    fn tracking_file(project_root: &PathBuf) -> PathBuf {
        project_root.join("wallet_tracking.json")
    }

    fn load(project_root: &PathBuf) -> Self {
        let file_path = Self::tracking_file(project_root);
        if let Ok(content) = fs::read_to_string(&file_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self, project_root: &PathBuf) -> Result<()> {
        let file_path = Self::tracking_file(project_root);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&file_path, content)?;
        Ok(())
    }

    fn get_initial_balance(&self, worker_id: usize) -> Option<f64> {
        self.initial_balances.get(&worker_id).copied()
    }

    fn set_initial_balance(&mut self, worker_id: usize, balance: f64) {
        self.initial_balances.insert(worker_id, balance);
    }
}

impl WalletManager {
    pub fn new() -> Result<Self> {
        let project_root = crate::utils::get_project_root()?;
        Ok(Self { project_root })
    }

    /// Get the gRPC endpoint for a wallet worker
    /// First checks docker inspect for port mappings, falls back to 8082 + worker_id
    fn get_wallet_endpoint(&self, worker_id: usize) -> String {
        let container_name = format!("kaswallet-{}", worker_id);

        // Try to get port mapping from docker inspect
        if let Ok(output) = Command::new("docker")
            .args(&["inspect", &container_name, "--format", "{{json .NetworkSettings.Ports}}"])
            .output()
        {
            if output.status.success() {
                let ports_json = String::from_utf8_lossy(&output.stdout);
                // Parse JSON to find the host port mapping for 8082/tcp
                if let Ok(ports_map) = serde_json::from_str::<Value>(&ports_json) {
                    if let Some(port_bindings) = ports_map.get("8082/tcp") {
                        if let Some(first_binding) = port_bindings.as_array().and_then(|a| a.first()) {
                            if let Some(host_port) = first_binding.get("HostPort").and_then(|p| p.as_str()) {
                                return format!("http://127.0.0.1:{}", host_port);
                            }
                        }
                    }
                }
            }
        }

        // Fallback to default port mapping: kaswallet-0 = 8082, kaswallet-1 = 8083, etc.
        let port = 8082 + worker_id;
        format!("http://127.0.0.1:{}", port)
    }

    /// Get wallet balance via gRPC
    pub async fn get_balance(&self, worker_id: usize) -> Result<f64> {
        let endpoint = self.get_wallet_endpoint(worker_id);

        // Create gRPC client with timeout
        let mut client = WalletClient::connect(endpoint.clone())
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

    /// Get wallet balance with per-address breakdown
    pub async fn get_balance_detailed(&self, worker_id: usize) -> Result<Vec<(String, f64, f64)>> {
        let endpoint = self.get_wallet_endpoint(worker_id);

        let mut client = WalletClient::connect(endpoint.clone())
            .await
            .context(format!("Failed to connect to kaswallet-daemon at {}", endpoint))?;

        let request = tonic::Request::new(kaswallet_proto::GetBalanceRequest {});
        let response = client.get_balance(request)
            .await
            .context("Failed to get balance from kaswallet-daemon")?;

        let balance_response = response.into_inner();

        // Parse address balances
        let address_balances: Vec<(String, f64, f64)> = balance_response
            .address_balances
            .into_iter()
            .map(|ab| {
                let available_kas = ab.available as f64 / 100_000_000.0;
                let pending_kas = ab.pending as f64 / 100_000_000.0;
                (ab.address, available_kas, pending_kas)
            })
            .collect();

        Ok(address_balances)
    }

    /// Get wallet address via gRPC (returns first address)
    pub async fn get_address(&self, worker_id: usize) -> Result<String> {
        let endpoint = self.get_wallet_endpoint(worker_id);

        // Create gRPC client
        let mut client = WalletClient::connect(endpoint.clone())
            .await
            .context(format!("Failed to connect to kaswallet-daemon at {}", endpoint))?;

        // Call GetAddresses RPC
        let request = tonic::Request::new(kaswallet_proto::GetAddressesRequest {});
        let response = client.get_addresses(request)
            .await
            .context("Failed to get addresses from kaswallet-daemon")?;

        let addresses_response = response.into_inner();

        // Return the first address, or error if no addresses
        addresses_response.address
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("No addresses found in wallet"))
    }

    /// Get UTXOs (Unspent Transaction Outputs) for wallet addresses via kaspad
    pub async fn get_utxos(&self, worker_id: usize) -> Result<Vec<UtxoInfo>> {
        use kaspa_wrpc_client::{
            client::{ConnectOptions, ConnectStrategy},
            prelude::NetworkType,
            KaspaRpcClient, WrpcEncoding,
        };
        use kaspa_rpc_core::api::rpc::RpcApi;
        use std::time::Duration;

        // Get wallet addresses first
        let addresses = match self.get_address(worker_id).await {
            Ok(addr) => vec![addr],
            Err(_) => return Ok(Vec::new()),
        };

        // Parse addresses to kaspa Address type
        let kaspa_addresses: Vec<kaspa_addresses::Address> = addresses
            .iter()
            .filter_map(|addr| {
                // Try to parse, log error but don't crash
                match addr.as_str().try_into() {
                    Ok(parsed_addr) => Some(parsed_addr),
                    Err(e) => {
                        eprintln!("Warning: Failed to parse address '{}': {}", addr, e);
                        None
                    }
                }
            })
            .collect();

        if kaspa_addresses.is_empty() {
            return Ok(Vec::new());
        }

        // Connect to kaspad via WRPC
        let encoding = WrpcEncoding::Borsh;
        let url = Some("ws://localhost:17210");
        let resolver = None;
        // For testnet, we need to specify the network suffix (testnet-11 is the recommended testnet)
        let network = Some(kaspa_wrpc_client::prelude::NetworkId::with_suffix(NetworkType::Testnet, 11));
        let subscription_context = None;

        let client = match KaspaRpcClient::new(encoding, url, resolver, network, subscription_context) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: Failed to create kaspad RPC client: {}", e);
                return Ok(Vec::new());
            }
        };

        let options = ConnectOptions {
            block_async_connect: true,
            connect_timeout: Some(Duration::from_millis(5000)),
            strategy: ConnectStrategy::Fallback,
            ..Default::default()
        };

        // Try to connect to kaspad
        if let Err(e) = client.connect(Some(options)).await {
            eprintln!("Warning: Failed to connect to kaspad: {}", e);
            return Ok(Vec::new());
        }

        // Get current DAG info for timestamp calculation
        let dag_info = client.get_block_dag_info().await.ok();

        // Query UTXOs for the addresses
        let utxos_result = client.get_utxos_by_addresses(kaspa_addresses).await;

        client.disconnect().await.ok();

        let utxo_entries = match utxos_result {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("Warning: Failed to get UTXOs from kaspad: {}", e);
                return Ok(Vec::new());
            }
        };

        // Calculate estimated timestamps if we have DAG info
        let (current_daa_score, current_time_ms) = if let Some(info) = dag_info {
            (info.virtual_daa_score, info.past_median_time)
        } else {
            (0, 0)
        };

        // Convert to our UtxoInfo format and try to fetch source addresses
        let mut utxos = Vec::new();
        for entry in utxo_entries {
            let utxo = entry.utxo_entry;
            let amount_kas = utxo.amount as f64 / 100_000_000.0;

            let address_str = entry.address
                .map(|a| a.to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            // Estimate timestamp: each DAA score ~= 1 second (approximate)
            let estimated_time_ms = if current_daa_score > 0 && current_time_ms > 0 {
                let daa_diff = current_daa_score.saturating_sub(utxo.block_daa_score);
                current_time_ms.saturating_sub(daa_diff * 1000) // milliseconds
            } else {
                0
            };

            // Try to get source address for non-coinbase transactions
            let source_addresses = if !utxo.is_coinbase {
                // Use get_utxo_return_address to get the source address
                let tx_id_rpc: kaspa_rpc_core::RpcHash = entry.outpoint.transaction_id.into();
                match client.get_utxo_return_address(tx_id_rpc, utxo.block_daa_score).await {
                    Ok(return_addr) => vec![return_addr.to_string()],
                    Err(e) => {
                        eprintln!("Failed to get source address for tx {}: {}", entry.outpoint.transaction_id, e);
                        Vec::new()
                    }
                }
            } else {
                Vec::new() // Coinbase transactions have no source
            };

            utxos.push(UtxoInfo {
                address: address_str,
                tx_id: entry.outpoint.transaction_id.to_string(),
                amount_kas,
                block_daa_score: utxo.block_daa_score,
                is_coinbase: utxo.is_coinbase,
                timestamp_ms: estimated_time_ms,
                source_addresses,
            });
        }

        // Sort by block_daa_score descending (most recent first)
        utxos.sort_by(|a, b| b.block_daa_score.cmp(&a.block_daa_score));

        Ok(utxos)
    }

    /// List all wallet information
    pub async fn list_wallets(&self) -> Result<Vec<WalletInfo>> {
        let mut wallets = Vec::new();

        // Load wallet tracking for fee calculation
        let mut tracking = WalletTracking::load(&self.project_root);
        let mut tracking_updated = false;

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

            // Calculate initial balance and fees spent
            let (initial_balance, fees_spent) = if let Some(current_balance) = balance {
                let initial = tracking.get_initial_balance(worker_id);

                // If no initial balance recorded, record current balance as initial
                let initial = if initial.is_none() {
                    tracking.set_initial_balance(worker_id, current_balance);
                    tracking_updated = true;
                    Some(current_balance)
                } else {
                    initial
                };

                // Calculate fees spent (initial - current)
                let spent = initial.map(|init| {
                    if init >= current_balance {
                        init - current_balance
                    } else {
                        // Balance increased (received funds), reset initial
                        tracking.set_initial_balance(worker_id, current_balance);
                        tracking_updated = true;
                        0.0
                    }
                });

                (initial, spent)
            } else {
                (None, None)
            };

            wallets.push(WalletInfo {
                worker_id,
                address,
                balance,
                container_running,
                initial_balance,
                fees_spent,
            });
        }

        // Save tracking if updated
        if tracking_updated {
            let _ = tracking.save(&self.project_root);
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
        let endpoint = self.get_wallet_endpoint(worker_id);

        // Create gRPC client
        let mut client = WalletClient::connect(endpoint.clone())
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
            transaction_description: String::new(), // Empty description
        });

        let response = client.send(request)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send transaction: {} (status: {:?})", e.message(), e.code()))?;

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

