// Test only the GetBalance method
use igra_cli::core::wallet::kaswallet_proto::wallet_client::WalletClient;
use igra_cli::core::wallet::kaswallet_proto;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Testing GetBalance RPC...\n");

    let endpoint = "http://127.0.0.1:8082";
    let mut client = WalletClient::connect(endpoint).await?;
    println!("✓ Connected to {}", endpoint);

    // Call GetBalance
    let request = tonic::Request::new(kaswallet_proto::GetBalanceRequest {});
    let response = client.get_balance(request).await?;
    let balance_response = response.into_inner();

    println!("✓ GetBalance succeeded!");
    println!("  Available: {} sompi ({} KAS)",
        balance_response.available,
        balance_response.available as f64 / 100_000_000.0
    );
    println!("  Pending: {} sompi", balance_response.pending);

    if !balance_response.address_balances.is_empty() {
        println!("\n  Address balances:");
        for ab in &balance_response.address_balances {
            println!("    {} - {} KAS", ab.address, ab.available as f64 / 100_000_000.0);
        }
    }

    Ok(())
}
