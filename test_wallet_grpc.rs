// Simple test to verify kaswallet gRPC integration
// Run with: cargo run --bin test_wallet_grpc

use igra_cli::core::wallet::WalletManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Testing kaswallet-daemon gRPC integration...\n");

    let wallet_manager = WalletManager::new()?;

    // Test worker 0 (kaswallet-0 on port 8082)
    println!("Testing kaswallet-0:");
    match wallet_manager.get_address(0).await {
        Ok(address) => {
            println!("  ✓ Address: {}", address);

            match wallet_manager.get_balance(0).await {
                Ok(balance) => {
                    println!("  ✓ Balance: {} KAS", balance);
                    println!("\n✅ gRPC integration successful!");
                }
                Err(e) => {
                    println!("  ✗ Balance error: {}", e);
                    eprintln!("\nDebug: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("  ✗ Address error: {}", e);
            eprintln!("\nDebug error: {:?}", e);
            println!("\n❌ Make sure kaswallet-0 is running!");
        }
    }

    Ok(())
}
