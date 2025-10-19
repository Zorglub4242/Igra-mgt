/// Standalone CLI for interacting with kaswallet-daemon
///
/// Usage: cargo run --bin kaswallet-cli -- [command] [args]

use anyhow::{Context, Result};
use std::env;

// Include the generated gRPC client code
pub mod kaswallet_proto {
    tonic::include_proto!("kaswallet_proto");
}

use kaswallet_proto::wallet_client::WalletClient;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];
    let worker_id: usize = if args.len() > 2 {
        args[2].parse().context("Worker ID must be a number (0-4)")?
    } else {
        0
    };

    let port = 8082 + worker_id;
    let endpoint = format!("http://127.0.0.1:{}", port);

    println!("🔗 Connecting to kaswallet-{} at {}", worker_id, endpoint);

    let mut client = WalletClient::connect(endpoint.clone())
        .await
        .context(format!("Failed to connect to kaswallet-daemon at {}", endpoint))?;

    match command.as_str() {
        "balance" => {
            println!("📊 Getting balance...");
            let request = tonic::Request::new(kaswallet_proto::GetBalanceRequest {});
            let response = client.get_balance(request).await?;
            let balance = response.into_inner();

            let balance_kas = balance.available as f64 / 100_000_000.0;
            println!("✓ Available: {:.8} KAS", balance_kas);
            println!("  Pending: {} sompi", balance.pending);
        }

        "address" | "addresses" => {
            println!("📬 Getting addresses...");
            let request = tonic::Request::new(kaswallet_proto::GetAddressesRequest {});
            let response = client.get_addresses(request).await?;
            let addresses = response.into_inner();

            println!("✓ Addresses:");
            for (i, addr) in addresses.address.iter().enumerate() {
                println!("  [{}] {}", i, addr);
            }
        }

        "send" => {
            if args.len() < 5 {
                eprintln!("❌ Usage: kaswallet-cli send <worker_id> <to_address> <amount_kas>");
                return Ok(());
            }

            let to_address = &args[3];
            let amount_kas: f64 = args[4].parse().context("Amount must be a number")?;
            let amount_sompi = (amount_kas * 100_000_000.0) as u64;

            println!("💸 Sending {} KAS to {}...", amount_kas, to_address);
            println!("⚠️  Warning: This requires wallet password");

            // Read password from stdin
            use std::io::{self, Write};
            print!("Enter password: ");
            io::stdout().flush()?;
            let mut password = String::new();
            io::stdin().read_line(&mut password)?;
            let password = password.trim().to_string();

            let request = tonic::Request::new(kaswallet_proto::SendRequest {
                to_address: to_address.to_string(),
                amount: amount_sompi,
                password,
                from: vec![],
                use_existing_change_address: false,
                is_send_all: false,
                fee_policy: None,
            });

            let response = client.send(request).await?;
            let result = response.into_inner();

            println!("✓ Transaction sent!");
            println!("  TxIDs: {}", result.tx_i_ds.join(", "));
            println!("  Signed {} transactions", result.signed_transactions.len());
        }

        "version" => {
            println!("📦 Getting version...");
            let request = tonic::Request::new(kaswallet_proto::GetVersionRequest {});
            let response = client.get_version(request).await?;
            let version = response.into_inner();

            println!("✓ Version: {}", version.version);
        }

        "help" | "--help" | "-h" => {
            print_usage();
        }

        _ => {
            eprintln!("❌ Unknown command: {}", command);
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("Kaswallet CLI - Interact with kaswallet-daemon containers");
    println!();
    println!("Usage:");
    println!("  cargo run --bin kaswallet-cli -- <command> [worker_id] [args]");
    println!();
    println!("Commands:");
    println!("  version [worker_id]                  - Get kaswallet-daemon version");
    println!("  balance [worker_id]                  - Get wallet balance (default worker 0)");
    println!("  address [worker_id]                  - Show wallet addresses");
    println!("  send [worker_id] <to> <amount>       - Send KAS to address");
    println!("  help                                 - Show this help");
    println!();
    println!("Examples:");
    println!("  cargo run --bin kaswallet-cli -- version 0");
    println!("  cargo run --bin kaswallet-cli -- balance 0");
    println!("  cargo run --bin kaswallet-cli -- address 0");
    println!("  cargo run --bin kaswallet-cli -- send 0 kaspatest:qq... 1.5");
    println!();
    println!("Worker IDs: 0-4 (corresponds to kaswallet-0 through kaswallet-4)");
    println!("Ports: 8082-8086");
}
