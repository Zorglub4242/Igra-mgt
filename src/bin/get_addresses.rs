/// Simple utility to get wallet addresses via gRPC
use igra_cli::core::wallet::kaswallet_proto::wallet_client::WalletClient;
use igra_cli::core::wallet::kaswallet_proto::NewAddressRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    for worker_id in 1..=4 {
        let port = 8082 + worker_id;
        let endpoint = format!("http://127.0.0.1:{}", port);

        match WalletClient::connect(endpoint.clone()).await {
            Ok(mut client) => {
                // Call NewAddress to generate an address if none exists
                match client.new_address(tonic::Request::new(NewAddressRequest {})).await {
                    Ok(response) => {
                        let address = response.into_inner().address;
                        println!("W{}_WALLET_TO_ADDRESS={}", worker_id, address);
                    }
                    Err(e) => eprintln!("Error getting new address for worker {}: {}", worker_id, e),
                }
            }
            Err(e) => eprintln!("Error connecting to worker {}: {}", worker_id, e),
        }
    }

    Ok(())
}
