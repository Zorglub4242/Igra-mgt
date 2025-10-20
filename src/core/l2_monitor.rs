/// L2 Transaction Monitoring
///
/// This module provides real-time monitoring of L2 transactions using:
/// - Reth metrics endpoint (port 9001) for statistics
/// - Ethereum JSON-RPC (port 9545) for transaction details
/// - Kaspa wallet UTXO tracking for L1 fee correlation

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ethers::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::wallet::{WalletManager, UtxoInfo};

const METRICS_URL: &str = "http://localhost:9001/metrics";
const RPC_URL: &str = "http://localhost:9545";

/// Transaction type classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    Transfer,
    Contract,
    Entry,
    Unknown,
}

impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionType::Transfer => write!(f, "TRANSFER"),
            TransactionType::Contract => write!(f, "CONTRACT"),
            TransactionType::Entry => write!(f, "ENTRY"),
            TransactionType::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Complete transaction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
    pub value: U256,
    pub gas_used: Option<U256>,
    pub gas_price: U256,
    pub block_number: u64,
    pub timestamp: DateTime<Utc>,
    pub status: bool,
    pub tx_type: TransactionType,
    pub l1_fee: Option<f64>, // KAS fee paid on L1 for entry transactions
}

impl TransactionInfo {
    /// Calculate gas fee in iKAS
    pub fn gas_fee_ikas(&self) -> f64 {
        if let Some(gas_used) = self.gas_used {
            let fee_wei = gas_used.saturating_mul(self.gas_price);
            wei_to_ikas(fee_wei)
        } else {
            0.0
        }
    }

    /// Format value in iKAS
    pub fn value_ikas(&self) -> f64 {
        wei_to_ikas(self.value)
    }
}

/// L2 network statistics
#[derive(Debug, Clone, Default)]
pub struct Statistics {
    pub current_block: u64,
    pub total_transactions: u64,
    pub successful_transactions: u64,
    pub failed_transactions: u64,
    pub total_gas_fees_ikas: f64,
    pub total_l1_fees_kas: f64,
    pub start_time: Option<DateTime<Utc>>,
    pub last_block_time: Option<DateTime<Utc>>,
}

impl Statistics {
    pub fn tps(&self) -> f64 {
        if let Some(start) = self.start_time {
            let elapsed = Utc::now().signed_duration_since(start).num_seconds() as f64;
            if elapsed > 0.0 {
                return self.total_transactions as f64 / elapsed;
            }
        }
        0.0
    }

    pub fn uptime(&self) -> String {
        if let Some(start) = self.start_time {
            let duration = Utc::now().signed_duration_since(start);
            let hours = duration.num_hours();
            let minutes = duration.num_minutes() % 60;
            format!("{}h {}m", hours, minutes)
        } else {
            String::from("0h 0m")
        }
    }
}

/// L1 fee tracker - correlates L1 wallet transactions with L2 entry transactions
pub struct L1FeeTracker {
    wallet_manager: WalletManager,
    l1_utxos: Arc<RwLock<Vec<UtxoInfo>>>,
    fee_cache: Arc<RwLock<HashMap<String, f64>>>, // tx_hash -> L1 fee in KAS
}

impl L1FeeTracker {
    pub fn new() -> Result<Self> {
        Ok(Self {
            wallet_manager: WalletManager::new()?,
            l1_utxos: Arc::new(RwLock::new(Vec::new())),
            fee_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Update L1 UTXO data from wallet (call periodically)
    pub async fn update_utxos(&self) -> Result<()> {
        // Fetch UTXOs from wallet-0 (the node's wallet for entry transactions)
        let utxos = self.wallet_manager.get_utxos(0).await?;

        let mut l1_utxos = self.l1_utxos.write().await;
        *l1_utxos = utxos;

        Ok(())
    }

    /// Get L1 fee for a transaction (if it's an entry transaction)
    pub async fn get_l1_fee(&self, tx_hash: &str, _value_ikas: f64) -> Option<f64> {
        // Check cache first
        {
            let cache = self.fee_cache.read().await;
            if let Some(fee) = cache.get(tx_hash) {
                return Some(*fee);
            }
        }

        // TODO: Implement correlation logic
        // This would match L1 transactions by:
        // 1. Timestamp correlation with L2 entry transaction
        // 2. Amount matching (KAS → iKAS, accounting for fees)
        // 3. UTXO analysis to find the transaction fee

        // For now, return None (to be implemented)
        None
    }
}

/// Main L2 transaction monitor
pub struct TransactionMonitor {
    provider: Provider<Http>,
    http_client: Client,
    l1_tracker: L1FeeTracker,
    statistics: Arc<RwLock<Statistics>>,
    last_block: Arc<RwLock<u64>>,
}

impl TransactionMonitor {
    pub async fn new() -> Result<Self> {
        let provider = Provider::<Http>::try_from(RPC_URL)
            .context("Failed to create Ethereum RPC provider")?;

        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;

        let l1_tracker = L1FeeTracker::new()?;

        Ok(Self {
            provider,
            http_client,
            l1_tracker,
            statistics: Arc::new(RwLock::new(Statistics::default())),
            last_block: Arc::new(RwLock::new(0)),
        })
    }

    /// Fetch and parse Prometheus metrics
    pub async fn fetch_metrics(&self) -> Result<HashMap<String, String>> {
        let response = self.http_client
            .get(METRICS_URL)
            .send()
            .await?
            .text()
            .await?;

        let mut metrics = HashMap::new();
        for line in response.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            if let Some((key, value)) = line.split_once(' ') {
                metrics.insert(key.to_string(), value.to_string());
            }
        }

        Ok(metrics)
    }

    /// Get current block number
    pub async fn get_block_number(&self) -> Result<u64> {
        let block = self.provider
            .get_block_number()
            .await
            .context("Failed to get block number")?;
        Ok(block.as_u64())
    }

    /// Fetch transactions from a specific block
    pub async fn fetch_block_transactions(&self, block_number: u64) -> Result<Vec<TransactionInfo>> {
        let block = self.provider
            .get_block_with_txs(BlockNumber::Number(block_number.into()))
            .await?
            .context(format!("Block {} not found", block_number))?;

        let mut transactions = Vec::new();

        for tx in block.transactions {
            // Fetch receipt for gas used and status
            let receipt = self.provider
                .get_transaction_receipt(tx.hash)
                .await?;

            let (gas_used, status) = if let Some(r) = receipt {
                (Some(r.gas_used.unwrap_or_default()), r.status == Some(1.into()))
            } else {
                (None, false)
            };

            // Classify transaction type
            let tx_type = classify_transaction(&tx);

            // Get L1 fee if it's an entry transaction
            let l1_fee = if tx_type == TransactionType::Entry {
                let value_ikas = wei_to_ikas(tx.value);
                self.l1_tracker.get_l1_fee(&format!("{:?}", tx.hash), value_ikas).await
            } else {
                None
            };

            let tx_info = TransactionInfo {
                hash: format!("{:?}", tx.hash),
                from: format!("{:?}", tx.from),
                to: tx.to.map(|addr| format!("{:?}", addr)),
                value: tx.value,
                gas_used,
                gas_price: tx.gas_price.unwrap_or_default(),
                block_number,
                timestamp: Utc::now(), // TODO: Get actual block timestamp
                status,
                tx_type,
                l1_fee,
            };

            transactions.push(tx_info);
        }

        Ok(transactions)
    }

    /// Update statistics with new transactions
    pub async fn update_statistics(&self, transactions: &[TransactionInfo]) {
        let mut stats = self.statistics.write().await;

        if stats.start_time.is_none() {
            stats.start_time = Some(Utc::now());
        }

        for tx in transactions {
            stats.total_transactions += 1;

            if tx.status {
                stats.successful_transactions += 1;
            } else {
                stats.failed_transactions += 1;
            }

            stats.total_gas_fees_ikas += tx.gas_fee_ikas();

            if let Some(l1_fee) = tx.l1_fee {
                stats.total_l1_fees_kas += l1_fee;
            }
        }

        if let Some(last_tx) = transactions.last() {
            stats.current_block = last_tx.block_number;
            stats.last_block_time = Some(last_tx.timestamp);
        }
    }

    /// Get current statistics
    pub async fn get_statistics(&self) -> Statistics {
        self.statistics.read().await.clone()
    }

    /// Poll for new blocks and transactions
    pub async fn poll_new_transactions(&self) -> Result<Vec<TransactionInfo>> {
        let current_block = self.get_block_number().await?;
        let mut last_block = self.last_block.write().await;

        if current_block <= *last_block {
            return Ok(Vec::new());
        }

        let mut all_transactions = Vec::new();

        // Fetch transactions from all new blocks
        for block_num in (*last_block + 1)..=current_block {
            match self.fetch_block_transactions(block_num).await {
                Ok(txs) => {
                    all_transactions.extend(txs);
                }
                Err(e) => {
                    eprintln!("Error fetching block {}: {}", block_num, e);
                }
            }
        }

        *last_block = current_block;

        // Update statistics
        self.update_statistics(&all_transactions).await;

        Ok(all_transactions)
    }

    /// Update L1 UTXO data
    pub async fn update_l1_data(&self) -> Result<()> {
        self.l1_tracker.update_utxos().await
    }
}

/// Classify transaction type based on transaction data
fn classify_transaction(tx: &Transaction) -> TransactionType {
    // Entry transactions typically have no 'to' address (contract creation)
    // or specific contract address with specific input data
    // TODO: Implement proper entry transaction detection based on your system

    if tx.to.is_none() {
        return TransactionType::Contract;
    }

    if !tx.input.is_empty() && tx.input.0.len() > 4 {
        // Has input data - likely contract interaction
        // Check if it matches entry transaction pattern
        // For now, just classify as contract
        TransactionType::Contract
    } else if tx.value.is_zero() {
        TransactionType::Contract
    } else {
        TransactionType::Transfer
    }
}

/// Convert Wei to iKAS (same as Ether)
fn wei_to_ikas(wei: U256) -> f64 {
    let eth_str = ethers::utils::format_units(wei, "ether").unwrap_or_else(|_| String::from("0"));
    eth_str.parse().unwrap_or(0.0)
}
