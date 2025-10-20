# Kaswallet gRPC Client Example

This is a standalone example showing how to connect to a kaswallet daemon via gRPC and perform common operations.

## Overview

The kaswallet daemon exposes a gRPC API on port 8082 (configurable). This example demonstrates how to:
- Connect to the wallet daemon
- Get wallet addresses
- Check balance
- Send transactions

## Architecture

```
Your Application
       |
       | gRPC (HTTP/2)
       v
kaswallet-daemon (port 8082)
       |
       | WRPC (Borsh encoding)
       v
   kaspad (Kaspa L1 node)
```

## Prerequisites

- Rust toolchain installed
- A running kaswallet-daemon instance (accessible on localhost:8082 or another port)
- A running kaspad instance (the wallet daemon connects to it)

## Project Structure

```
wallet-grpc-client/
├── README.md           # This file
├── Cargo.toml         # Rust project configuration
├── build.rs           # Build script to compile protobuf
├── proto/
│   └── kaspawalletd.proto  # gRPC service definition
└── src/
    └── main.rs        # Example client code
```

## Setup

### 1. Copy the Proto File

The protobuf definition is already included in `proto/kaspawalletd.proto`. This defines the gRPC service interface for the kaswallet daemon.

### 2. Build the Project

```bash
cd wallet-grpc-client
cargo build --release
```

This will:
- Compile the protobuf file to Rust code
- Build the example client

### 3. Run the Example

Make sure you have a kaswallet daemon running on `localhost:8082` (default), then:

```bash
cargo run --release
```

Or to connect to a different port:

```bash
# Edit src/main.rs to change the endpoint
# Then rebuild and run
cargo run --release
```

## Usage Examples

### Basic Operations

The example code in `src/main.rs` demonstrates:

1. **Get New Address**
   ```rust
   let response = client.new_address(Request::new(NewAddressRequest {})).await?;
   println!("Address: {}", response.into_inner().address);
   ```

2. **Get All Addresses**
   ```rust
   let response = client.get_addresses(Request::new(GetAddressesRequest {})).await?;
   for addr in response.into_inner().address {
       println!("Address: {}", addr);
   }
   ```

3. **Get Balance**
   ```rust
   let response = client.get_balance(Request::new(GetBalanceRequest {})).await?;
   let balance = response.into_inner();
   println!("Available: {} sompi", balance.available);
   println!("Pending: {} sompi", balance.pending);
   ```

4. **Send Transaction**
   ```rust
   let response = client.send(Request::new(SendRequest {
       to_address: "kaspatest:qq...".to_string(),
       amount: 100_000_000, // 1 KAS in sompi
       password: "your_password".to_string(),
       from: vec![],
       use_existing_change_address: false,
       is_send_all: false,
       fee_policy: None,
   })).await?;
   ```

## gRPC Service Definition

The wallet daemon exposes the following main RPC methods:

- `NewAddress()` - Generate a new receive address
- `GetAddresses()` - Get all wallet addresses
- `GetBalance()` - Get wallet balance (available + pending)
- `Send()` - Send KAS to an address
- `Sign()` - Sign a transaction
- `Broadcast()` - Broadcast a signed transaction

See `proto/kaspawalletd.proto` for the complete API definition.

## Currency Conversion

Kaspa uses "sompi" as the base unit:
- 1 KAS = 100,000,000 sompi (10^8)
- To convert KAS to sompi: multiply by 100_000_000
- To convert sompi to KAS: divide by 100_000_000.0

Example:
```rust
let amount_kas = 1.5;
let amount_sompi = (amount_kas * 100_000_000.0) as u64;  // 150_000_000 sompi

let balance_sompi = 250_000_000u64;
let balance_kas = balance_sompi as f64 / 100_000_000.0;  // 2.5 KAS
```

## Port Configuration

By default, kaswallet daemons listen on:
- kaswallet-0: `localhost:8082`
- kaswallet-1: `localhost:8083`
- kaswallet-2: `localhost:8084`
- kaswallet-3: `localhost:8085`
- kaswallet-4: `localhost:8086`

To connect to a different worker, change the endpoint in `src/main.rs`:
```rust
let endpoint = "http://127.0.0.1:8083"; // Connect to kaswallet-1
```

## Error Handling

The example uses `anyhow::Result` for simple error handling. In production code, you should:
- Handle specific gRPC error codes
- Implement retry logic for network failures
- Validate addresses before sending
- Check balance before attempting transactions

## Integration with Your Code

To integrate this into your own Rust project:

1. **Add dependencies to your `Cargo.toml`**:
   ```toml
   [dependencies]
   tonic = "0.12"
   prost = "0.13"
   tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
   anyhow = "1.0"

   [build-dependencies]
   tonic-build = "0.12"
   ```

2. **Copy the proto file** to your project's `proto/` directory

3. **Create `build.rs`** in your project root:
   ```rust
   fn main() -> Result<(), Box<dyn std::error::Error>> {
       tonic_build::configure()
           .build_server(false)
           .compile(&["proto/kaspawalletd.proto"], &["proto"])?;
       Ok(())
   }
   ```

4. **Use the generated client** in your code:
   ```rust
   pub mod kaswallet_proto {
       tonic::include_proto!("kaswallet_proto");
   }

   use kaswallet_proto::wallet_client::WalletClient;
   ```

## Troubleshooting

### Connection Refused
- Check that kaswallet-daemon is running: `docker ps | grep kaswallet`
- Verify the port is correct: `docker port kaswallet-0`
- Ensure the daemon has finished initializing (check logs)

### Invalid Address Error
- Testnet addresses start with `kaspatest:`
- Mainnet addresses start with `kaspa:`
- Make sure you're using the correct network prefix

### Insufficient Balance
- Check wallet balance first with `GetBalance`
- Remember to account for transaction fees
- Ensure the wallet has received and confirmed UTXOs

## Additional Resources

- [Kaspa Developer Documentation](https://kaspa.org/developers/)
- [gRPC Documentation](https://grpc.io/docs/)
- [Tonic (Rust gRPC) Documentation](https://docs.rs/tonic/)
- [Protobuf Documentation](https://developers.google.com/protocol-buffers)

## License

This example code is provided as-is for educational purposes.
