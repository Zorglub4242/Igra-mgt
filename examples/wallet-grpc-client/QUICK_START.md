# Quick Start Guide

Get up and running with the Kaswallet gRPC client in 3 minutes.

## Prerequisites

- Rust installed (https://rustup.rs/)
- A running kaswallet daemon (part of IGRA Orchestra)

## Step 1: Clone or Copy This Example

```bash
# If you're in the igra-orchestra repository
cd tools/igra-cli/examples/wallet-grpc-client

# Or copy this entire folder to your own project
cp -r wallet-grpc-client /path/to/your/project/
cd /path/to/your/project/wallet-grpc-client
```

## Step 2: Build the Example

```bash
cargo build --release
```

This will:
- Download dependencies (tonic, prost, tokio)
- Compile the protobuf file to Rust code
- Build the example binary

## Step 3: Run the Example

Make sure you have a kaswallet daemon running, then:

```bash
cargo run --release
```

You should see output like:

```
=== Kaswallet gRPC Client Example ===

Connecting to kaswallet-0 at http://127.0.0.1:8082...
✓ Connected successfully

--- Example 1: Generate New Address ---
✓ New address: kaspatest:qq...

--- Example 2: Get All Addresses ---
✓ Found 2 address(es):
  1. kaspatest:qq...
  2. kaspatest:qr...

--- Example 3: Get Wallet Balance ---
✓ Balance:
  Available: 100.5 KAS (10050000000 sompi)
  Pending:   0 KAS (0 sompi)

...
```

## Step 4: Customize for Your Use Case

Edit `src/main.rs` to:

1. **Connect to a different worker** (line 19):
   ```rust
   let worker_id = 1; // Change to 1, 2, 3, or 4
   ```

2. **Send a transaction** (line 105):
   - Uncomment the code block
   - Set the destination address
   - Set the amount in KAS
   - Provide the wallet password

3. **Add your own functionality**:
   ```rust
   // Example: Get balance periodically
   loop {
       let response = client.get_balance(Request::new(GetBalanceRequest {})).await?;
       println!("Balance: {} KAS", response.into_inner().available as f64 / 1e8);
       tokio::time::sleep(Duration::from_secs(10)).await;
   }
   ```

## Common Tasks

### Connect to a Different Port

```rust
let endpoint = "http://127.0.0.1:8083"; // kaswallet-1
```

### Send a Transaction

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

println!("TxID: {}", response.into_inner().tx_i_ds[0]);
```

### Check if Wallet Has Enough Balance

```rust
let min_amount_sompi = 100_000_000u64; // 1 KAS
let balance = client.get_balance(Request::new(GetBalanceRequest {})).await?;

if balance.into_inner().available >= min_amount_sompi {
    println!("Sufficient balance");
} else {
    println!("Insufficient balance");
}
```

## Integration into Your Project

To use this in your own Rust project:

1. **Copy the dependencies** from `Cargo.toml` to your project's `Cargo.toml`

2. **Copy the build script** `build.rs` to your project root

3. **Copy the proto file** `proto/kaspawalletd.proto` to your project

4. **Import and use** the generated client:
   ```rust
   pub mod kaswallet_proto {
       tonic::include_proto!("kaswallet_proto");
   }

   use kaswallet_proto::wallet_client::WalletClient;
   ```

## Troubleshooting

### "Connection refused"
- Check kaswallet is running: `docker ps | grep kaswallet`
- Verify the port: `docker port kaswallet-0 8082`

### "No addresses found"
- Generate one first: Call `NewAddress` RPC

### "Insufficient balance"
- Check balance with `GetBalance` RPC
- Send some test KAS to the wallet address

## Next Steps

- Read the full [README.md](README.md) for detailed API documentation
- Explore the protobuf definition in `proto/kaspawalletd.proto`
- Check out the [gRPC documentation](https://grpc.io/docs/)

## Support

For questions or issues:
- Check the IGRA Orchestra documentation
- Review the example code in `src/main.rs`
- Ask in the IGRA community channels
