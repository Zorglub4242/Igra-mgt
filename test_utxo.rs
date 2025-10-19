// Test UTXO querying from kaspad
use igra_cli::core::wallet::kaswallet_proto::wallet_client::WalletClient;
use igra_cli::core::wallet::kaswallet_proto;
use kaspa_wrpc_client::{
    client::{ConnectOptions, ConnectStrategy},
    prelude::NetworkType,
    KaspaRpcClient, WrpcEncoding,
};
use kaspa_rpc_core::api::rpc::RpcApi;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Testing UTXO query from kaspad...\n");

    // Step 1: Get address from kaswallet-0
    let endpoint = "http://127.0.0.1:8082";
    println!("1. Connecting to kaswallet at {}...", endpoint);
    let mut wallet_client = WalletClient::connect(endpoint).await?;
    println!("   ✓ Connected");

    // Get addresses
    let request = tonic::Request::new(kaswallet_proto::GetAddressesRequest {});
    let response = wallet_client.get_addresses(request).await?;
    let addresses_response = response.into_inner();

    if addresses_response.address.is_empty() {
        println!("   ✗ No addresses found in wallet");
        return Ok(());
    }

    let wallet_address = &addresses_response.address[0];
    println!("   ✓ Got wallet address: {}", wallet_address);

    // Step 2: Parse address to kaspa Address type
    println!("\n2. Parsing address to kaspa_addresses::Address...");
    let kaspa_address: kaspa_addresses::Address = match wallet_address.as_str().try_into() {
        Ok(addr) => {
            println!("   ✓ Successfully parsed address");
            addr
        }
        Err(e) => {
            println!("   ✗ Failed to parse address: {}", e);
            println!("     Address string: '{}'", wallet_address);
            println!("     Error type: {:?}", e);
            return Err(e.into());
        }
    };

    println!("   Parsed address details:");
    println!("     Prefix: {:?}", kaspa_address.prefix);
    println!("     Version: {:?}", kaspa_address.version);

    // Step 3: Connect to kaspad
    println!("\n3. Connecting to kaspad via WRPC...");
    let encoding = WrpcEncoding::Borsh;
    let url = Some("ws://localhost:17210");
    let resolver = None;
    // For testnet, we need to specify the network suffix (testnet-11 is the recommended testnet)
    let network = Some(kaspa_wrpc_client::prelude::NetworkId::with_suffix(NetworkType::Testnet, 11));
    let subscription_context = None;

    let client = KaspaRpcClient::new(encoding, url, resolver, network, subscription_context)?;
    println!("   ✓ Created kaspad RPC client");

    let options = ConnectOptions {
        block_async_connect: true,
        connect_timeout: Some(Duration::from_millis(5000)),
        strategy: ConnectStrategy::Fallback,
        ..Default::default()
    };

    client.connect(Some(options)).await?;
    println!("   ✓ Connected to kaspad");

    // Step 4: Get DAG info for timestamp estimation
    println!("\n4. Getting current DAG info for timestamp estimation...");
    let dag_info = client.get_block_dag_info().await?;
    println!("   ✓ Current DAA Score: {}", dag_info.virtual_daa_score);
    println!("   ✓ Current Time: {} ms", dag_info.past_median_time);

    // Step 5: Query UTXOs
    println!("\n5. Querying UTXOs for address...");
    let utxos_result = client.get_utxos_by_addresses(vec![kaspa_address]).await?;
    println!("   ✓ Received {} UTXO entries", utxos_result.len());

    // Step 6: Display results
    if utxos_result.is_empty() {
        println!("\n   No UTXOs found (wallet has no funds or no transactions)");
    } else {
        println!("\n   UTXOs:");
        for (i, entry) in utxos_result.iter().enumerate() {
            let utxo = &entry.utxo_entry;
            let amount_kas = utxo.amount as f64 / 100_000_000.0;

            // Calculate estimated timestamp
            let daa_diff = dag_info.virtual_daa_score.saturating_sub(utxo.block_daa_score);
            let estimated_time_ms = dag_info.past_median_time.saturating_sub(daa_diff * 1000);
            let timestamp_str = if estimated_time_ms > 0 {
                let secs = estimated_time_ms / 1000;
                if let Some(dt) = chrono::DateTime::from_timestamp(secs as i64, 0) {
                    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
                } else {
                    "Unknown".to_string()
                }
            } else {
                "Unknown".to_string()
            };

            println!("   [{}]", i + 1);
            println!("     Date/Time: {}", timestamp_str);
            println!("     Transaction ID: {}", entry.outpoint.transaction_id);
            println!("     Amount: {} KAS", amount_kas);
            println!("     Block DAA Score: {}", utxo.block_daa_score);
            println!("     Is Coinbase: {}", utxo.is_coinbase);
            if let Some(addr) = &entry.address {
                println!("     Address: {}", addr);
            }
        }
    }

    client.disconnect().await?;
    println!("\n✓ Test completed successfully!");

    Ok(())
}
