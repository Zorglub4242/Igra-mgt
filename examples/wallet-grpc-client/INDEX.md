# Kaswallet gRPC Client - Complete Guide

This directory contains a complete, standalone example showing how to connect to a kaswallet daemon via gRPC.

## 📁 What's Included

```
wallet-grpc-client/
├── README.md              # Complete documentation with API reference
├── QUICK_START.md         # Get running in 3 minutes
├── PYTHON_EXAMPLE.md      # Python implementation guide
├── INDEX.md               # This file
├── Cargo.toml            # Rust dependencies
├── build.rs              # Build script for proto compilation
├── .gitignore            # Git ignore rules
├── proto/
│   └── kaspawalletd.proto  # gRPC service definition
└── src/
    └── main.rs           # Working Rust example
```

## 🚀 Quick Start

**For Rust developers:**
```bash
cd wallet-grpc-client
cargo run --release
```

**For Python developers:**
See [PYTHON_EXAMPLE.md](PYTHON_EXAMPLE.md)

## 📖 Documentation Files

### [QUICK_START.md](QUICK_START.md)
- 3-minute setup guide
- Basic usage examples
- Common tasks (send, balance check)
- Integration steps

**Start here if:** You want to get running quickly

### [README.md](README.md)
- Complete architecture overview
- Full API reference
- Detailed examples
- Troubleshooting guide
- Port configurations

**Start here if:** You need comprehensive documentation

### [PYTHON_EXAMPLE.md](PYTHON_EXAMPLE.md)
- Python client implementation
- grpcio usage
- Async/await patterns
- Python-specific tips

**Start here if:** You're using Python

## 🎯 Use Cases

This example is perfect for:

1. **Learning** how to connect to kaswallet via gRPC
2. **Building** your own wallet integration
3. **Testing** wallet operations programmatically
4. **Reference** implementation for other languages

## 🔑 Key Features Demonstrated

- ✅ Connect to kaswallet daemon
- ✅ Generate new addresses
- ✅ Get all wallet addresses
- ✅ Check balance (with per-address breakdown)
- ✅ Send transactions (code included but commented for safety)
- ✅ Currency conversion (KAS ↔ sompi)
- ✅ Error handling
- ✅ Multi-worker support (kaswallets 0-4)

## 🛠️ How It Works

```
Your Application
       |
       | gRPC (HTTP/2)
       | Port: 8082-8086
       v
kaswallet-daemon
       |
       | WRPC (Borsh)
       | Port: 17210
       v
   kaspad (Kaspa L1)
```

The example uses:
- **tonic** - Rust gRPC framework
- **prost** - Protocol Buffers implementation
- **tokio** - Async runtime

## 📝 Code Examples

### Rust

```rust
use kaswallet_proto::wallet_client::WalletClient;

let mut client = WalletClient::connect("http://127.0.0.1:8082").await?;
let response = client.get_balance(Request::new(GetBalanceRequest {})).await?;
println!("Balance: {} KAS", response.into_inner().available as f64 / 1e8);
```

### Python

```python
import grpc
import kaspawalletd_pb2_grpc

channel = grpc.insecure_channel("127.0.0.1:8082")
client = kaspawalletd_pb2_grpc.WalletStub(channel)
response = client.GetBalance(kaspawalletd_pb2.GetBalanceRequest())
print(f"Balance: {response.available / 1e8} KAS")
```

## 🔧 Configuration

### Connect to Different Workers

**Worker 0** (default): `http://127.0.0.1:8082`
**Worker 1**: `http://127.0.0.1:8083`
**Worker 2**: `http://127.0.0.1:8084`
**Worker 3**: `http://127.0.0.1:8085`
**Worker 4**: `http://127.0.0.1:8086`

Change in code:
```rust
let worker_id = 1; // Connect to worker 1
let endpoint = format!("http://127.0.0.1:{}", 8082 + worker_id);
```

## 💡 Integration Guide

### Adding to Your Rust Project

1. Copy dependencies from `Cargo.toml`
2. Copy `build.rs` to your project root
3. Copy `proto/kaspawalletd.proto`
4. Import in your code:
   ```rust
   pub mod kaswallet_proto {
       tonic::include_proto!("kaswallet_proto");
   }
   ```

### Adding to Your Python Project

1. Install: `pip install grpcio grpcio-tools`
2. Copy `proto/kaspawalletd.proto`
3. Generate code:
   ```bash
   python -m grpc_tools.protoc -I./proto \
       --python_out=. --grpc_python_out=. \
       proto/kaspawalletd.proto
   ```

## 🔍 Available RPC Methods

From `proto/kaspawalletd.proto`:

- `NewAddress()` - Generate receive address
- `GetAddresses()` - List all addresses
- `GetBalance()` - Get balance with per-address breakdown
- `Send()` - Send KAS transaction
- `Sign()` - Sign transaction
- `Broadcast()` - Broadcast signed transaction
- `EstimateNetworkFee()` - Estimate transaction fee
- `GetUtxos()` - Get unspent outputs

See the proto file for complete definitions.

## 🧪 Testing

The example is fully tested and working:

```bash
cd wallet-grpc-client
cargo run --release
```

Expected output:
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

## 🐛 Troubleshooting

### Connection Refused
- Ensure kaswallet daemon is running: `docker ps | grep kaswallet`
- Check port: `docker port kaswallet-0 8082`

### No Addresses Found
- Generate one with `NewAddress` RPC

### Insufficient Balance
- Send test KAS to wallet address
- Check balance with `GetBalance`

## 📚 Additional Resources

- [IGRA Orchestra Documentation](../../README.md)
- [gRPC Official Docs](https://grpc.io/docs/)
- [Tonic (Rust gRPC)](https://docs.rs/tonic/)
- [Protocol Buffers](https://developers.google.com/protocol-buffers)

## 🤝 Support

For questions or issues:
1. Review the documentation files in this directory
2. Check the working example in `src/main.rs`
3. Ask in IGRA community channels

## 📄 License

This example code is provided as-is for educational purposes.

---

**Note for IGRA Team:** This standalone example can be shared with developers who need to integrate with kaswallet. All necessary files are included - just copy this entire directory.
