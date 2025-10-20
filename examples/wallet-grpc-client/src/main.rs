/// Kaswallet gRPC Client Example
///
/// This example demonstrates how to connect to a kaswallet daemon via gRPC
/// and perform common wallet operations.

use anyhow::Result;

// Include the generated gRPC client code from kaspawalletd.proto
pub mod kaswallet_proto {
    tonic::include_proto!("kaswallet_proto");
}

use kaswallet_proto::wallet_client::WalletClient;
use kaswallet_proto::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Kaswallet gRPC Client Example ===\n");

    // Configuration
    let worker_id = 0; // Change this to connect to different workers (0-4)
    let endpoint = format!("http://127.0.0.1:{}", 8082 + worker_id);

    println!("Connecting to kaswallet-{} at {}...", worker_id, endpoint);

    // Create gRPC client
    let mut client = WalletClient::connect(endpoint).await?;
    println!("✓ Connected successfully\n");

    // Example 1: Generate a new address
    println!("--- Example 1: Generate New Address ---");
    match client.new_address(tonic::Request::new(NewAddressRequest {})).await {
        Ok(response) => {
            let address = response.into_inner().address;
            println!("✓ New address: {}\n", address);
        }
        Err(e) => {
            println!("✗ Failed to generate address: {}\n", e);
        }
    }

    // Example 2: Get all addresses
    println!("--- Example 2: Get All Addresses ---");
    match client.get_addresses(tonic::Request::new(GetAddressesRequest {})).await {
        Ok(response) => {
            let addresses = response.into_inner().address;
            println!("✓ Found {} address(es):", addresses.len());
            for (i, addr) in addresses.iter().enumerate() {
                println!("  {}. {}", i + 1, addr);
            }
            println!();
        }
        Err(e) => {
            println!("✗ Failed to get addresses: {}\n", e);
        }
    }

    // Example 3: Get balance
    println!("--- Example 3: Get Wallet Balance ---");
    match client.get_balance(tonic::Request::new(GetBalanceRequest {})).await {
        Ok(response) => {
            let balance = response.into_inner();

            // Convert sompi to KAS (1 KAS = 100,000,000 sompi)
            let available_kas = balance.available as f64 / 100_000_000.0;
            let pending_kas = balance.pending as f64 / 100_000_000.0;

            println!("✓ Balance:");
            println!("  Available: {} KAS ({} sompi)", available_kas, balance.available);
            println!("  Pending:   {} KAS ({} sompi)", pending_kas, balance.pending);

            // Show per-address breakdown
            if !balance.address_balances.is_empty() {
                println!("\n  Per-address breakdown:");
                for addr_balance in balance.address_balances {
                    let addr_available = addr_balance.available as f64 / 100_000_000.0;
                    let addr_pending = addr_balance.pending as f64 / 100_000_000.0;
                    println!("    {}", addr_balance.address);
                    println!("      Available: {} KAS", addr_available);
                    println!("      Pending:   {} KAS", addr_pending);
                }
            }
            println!();
        }
        Err(e) => {
            println!("✗ Failed to get balance: {}\n", e);
        }
    }

    // Example 4: Send transaction (commented out for safety)
    println!("--- Example 4: Send Transaction ---");
    println!("(This example is commented out for safety)");
    println!("To send a transaction, uncomment the code below and provide:");
    println!("  - Destination address");
    println!("  - Amount in sompi (1 KAS = 100,000,000 sompi)");
    println!("  - Wallet password");
    println!();

    /*
    // UNCOMMENT AND CONFIGURE TO SEND A TRANSACTION
    let destination = "kaspatest:qq...".to_string(); // Replace with actual address
    let amount_kas = 1.0; // Amount in KAS
    let amount_sompi = (amount_kas * 100_000_000.0) as u64;
    let password = "your_password".to_string(); // Replace with actual password

    println!("Sending {} KAS ({} sompi) to {}...", amount_kas, amount_sompi, destination);

    match client.send(tonic::Request::new(SendRequest {
        to_address: destination,
        amount: amount_sompi,
        password,
        from: vec![], // Use default source addresses
        use_existing_change_address: false,
        is_send_all: false,
        fee_policy: None, // Use default fee policy
    })).await {
        Ok(response) => {
            let send_response = response.into_inner();
            println!("✓ Transaction sent!");
            println!("  Transaction IDs: {}", send_response.tx_i_ds.join(", "));
            println!("  Signed {} transaction(s)", send_response.signed_transactions.len());
            println!();
        }
        Err(e) => {
            println!("✗ Failed to send transaction: {}\n", e);
        }
    }
    */

    // Example 5: Helper function to convert between KAS and sompi
    println!("--- Example 5: Currency Conversion ---");
    let kas_amount = 2.5;
    let sompi_amount = kas_to_sompi(kas_amount);
    println!("{} KAS = {} sompi", kas_amount, sompi_amount);

    let sompi_amount = 150_000_000u64;
    let kas_amount = sompi_to_kas(sompi_amount);
    println!("{} sompi = {} KAS", sompi_amount, kas_amount);
    println!();

    println!("=== Example Complete ===");
    Ok(())
}

/// Convert KAS to sompi (1 KAS = 100,000,000 sompi)
fn kas_to_sompi(kas: f64) -> u64 {
    (kas * 100_000_000.0) as u64
}

/// Convert sompi to KAS (1 KAS = 100,000,000 sompi)
fn sompi_to_kas(sompi: u64) -> f64 {
    sompi as f64 / 100_000_000.0
}
