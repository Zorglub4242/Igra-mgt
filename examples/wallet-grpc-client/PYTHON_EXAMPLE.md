# Python gRPC Client Example

For teams using Python, here's how to connect to the kaswallet daemon using Python and grpcio.

## Installation

```bash
pip install grpcio grpcio-tools
```

## Generate Python Code from Proto

```bash
# From the wallet-grpc-client directory
python -m grpc_tools.protoc \
    -I./proto \
    --python_out=. \
    --grpc_python_out=. \
    proto/kaspawalletd.proto
```

This creates:
- `kaspawalletd_pb2.py` - Message definitions
- `kaspawalletd_pb2_grpc.py` - Service client

## Example Client Code

Save as `wallet_client.py`:

```python
#!/usr/bin/env python3
"""
Kaswallet gRPC Client Example (Python)

This example demonstrates how to connect to a kaswallet daemon
and perform common operations using Python.
"""

import grpc
import kaspawalletd_pb2
import kaspawalletd_pb2_grpc


def kas_to_sompi(kas: float) -> int:
    """Convert KAS to sompi (1 KAS = 100,000,000 sompi)"""
    return int(kas * 100_000_000)


def sompi_to_kas(sompi: int) -> float:
    """Convert sompi to KAS"""
    return sompi / 100_000_000.0


def main():
    print("=== Kaswallet gRPC Client Example (Python) ===\n")

    # Configuration
    worker_id = 0
    endpoint = f"127.0.0.1:{8082 + worker_id}"

    print(f"Connecting to kaswallet-{worker_id} at {endpoint}...")

    # Create gRPC channel and client
    channel = grpc.insecure_channel(endpoint)
    client = kaspawalletd_pb2_grpc.WalletStub(channel)

    print("✓ Connected successfully\n")

    # Example 1: Generate new address
    print("--- Example 1: Generate New Address ---")
    try:
        request = kaspawalletd_pb2.NewAddressRequest()
        response = client.NewAddress(request)
        print(f"✓ New address: {response.address}\n")
    except grpc.RpcError as e:
        print(f"✗ Failed: {e.details()}\n")

    # Example 2: Get all addresses
    print("--- Example 2: Get All Addresses ---")
    try:
        request = kaspawalletd_pb2.GetAddressesRequest()
        response = client.GetAddresses(request)
        print(f"✓ Found {len(response.address)} address(es):")
        for i, addr in enumerate(response.address, 1):
            print(f"  {i}. {addr}")
        print()
    except grpc.RpcError as e:
        print(f"✗ Failed: {e.details()}\n")

    # Example 3: Get balance
    print("--- Example 3: Get Wallet Balance ---")
    try:
        request = kaspawalletd_pb2.GetBalanceRequest()
        response = client.GetBalance(request)

        available_kas = sompi_to_kas(response.available)
        pending_kas = sompi_to_kas(response.pending)

        print("✓ Balance:")
        print(f"  Available: {available_kas:.8f} KAS ({response.available} sompi)")
        print(f"  Pending:   {pending_kas:.8f} KAS ({response.pending} sompi)")

        if response.addressBalances:
            print("\n  Per-address breakdown:")
            for addr_balance in response.addressBalances:
                addr_available = sompi_to_kas(addr_balance.available)
                addr_pending = sompi_to_kas(addr_balance.pending)
                print(f"    {addr_balance.address}")
                print(f"      Available: {addr_available:.8f} KAS")
                print(f"      Pending:   {addr_pending:.8f} KAS")
        print()
    except grpc.RpcError as e:
        print(f"✗ Failed: {e.details()}\n")

    # Example 4: Send transaction (commented for safety)
    print("--- Example 4: Send Transaction ---")
    print("(Example commented out for safety)")
    print("Uncomment the code below to send a transaction:\n")

    # UNCOMMENT TO SEND TRANSACTION
    """
    try:
        request = kaspawalletd_pb2.SendRequest(
            toAddress="kaspatest:qq...",  # Replace with destination
            amount=kas_to_sompi(1.0),     # 1 KAS
            password="your_password",      # Replace with wallet password
            from_=[],
            useExistingChangeAddress=False,
            isSendAll=False,
        )
        response = client.Send(request)
        print("✓ Transaction sent!")
        print(f"  TxIDs: {', '.join(response.txIDs)}")
        print(f"  Signed {len(response.signedTransactions)} transaction(s)\n")
    except grpc.RpcError as e:
        print(f"✗ Failed: {e.details()}\n")
    """

    # Example 5: Currency conversion
    print("--- Example 5: Currency Conversion ---")
    kas_amount = 2.5
    sompi_amount = kas_to_sompi(kas_amount)
    print(f"{kas_amount} KAS = {sompi_amount} sompi")

    sompi_amount = 150_000_000
    kas_amount = sompi_to_kas(sompi_amount)
    print(f"{sompi_amount} sompi = {kas_amount} KAS")
    print()

    print("=== Example Complete ===")
    channel.close()


if __name__ == "__main__":
    main()
```

## Running the Example

```bash
# Generate Python code from proto (only needed once)
python -m grpc_tools.protoc -I./proto --python_out=. --grpc_python_out=. proto/kaspawalletd.proto

# Run the example
python wallet_client.py
```

## Expected Output

```
=== Kaswallet gRPC Client Example (Python) ===

Connecting to kaswallet-0 at 127.0.0.1:8082...
✓ Connected successfully

--- Example 1: Generate New Address ---
✓ New address: kaspatest:qq...

--- Example 2: Get All Addresses ---
✓ Found 2 address(es):
  1. kaspatest:qq...
  2. kaspatest:qr...

--- Example 3: Get Wallet Balance ---
✓ Balance:
  Available: 100.50000000 KAS (10050000000 sompi)
  Pending:   0.00000000 KAS (0 sompi)
...
```

## Common Python Patterns

### Async/Await Support

For async Python code:

```bash
pip install grpcio-tools aiogrpc
```

```python
import grpc.aio
import kaspawalletd_pb2
import kaspawalletd_pb2_grpc

async def get_balance_async():
    async with grpc.aio.insecure_channel("127.0.0.1:8082") as channel:
        client = kaspawalletd_pb2_grpc.WalletStub(channel)
        request = kaspawalletd_pb2.GetBalanceRequest()
        response = await client.GetBalance(request)
        return response.available / 1e8
```

### Error Handling

```python
try:
    response = client.GetBalance(request)
except grpc.RpcError as e:
    if e.code() == grpc.StatusCode.UNAVAILABLE:
        print("Wallet daemon is not running")
    elif e.code() == grpc.StatusCode.UNAUTHENTICATED:
        print("Authentication failed")
    else:
        print(f"Error: {e.details()}")
```

### Connection with Timeout

```python
channel = grpc.insecure_channel(
    "127.0.0.1:8082",
    options=[
        ('grpc.max_send_message_length', 50 * 1024 * 1024),
        ('grpc.max_receive_message_length', 50 * 1024 * 1024),
    ]
)

# Call with deadline
response = client.GetBalance(
    request,
    timeout=5.0  # 5 second timeout
)
```

## Integration Tips

1. **Add to requirements.txt**:
   ```
   grpcio>=1.60.0
   grpcio-tools>=1.60.0
   ```

2. **Regenerate on proto changes**:
   ```bash
   python -m grpc_tools.protoc \
       -I./proto \
       --python_out=. \
       --grpc_python_out=. \
       proto/kaspawalletd.proto
   ```

3. **Use in your application**:
   ```python
   from wallet_client import kas_to_sompi, sompi_to_kas
   import kaspawalletd_pb2
   import kaspawalletd_pb2_grpc

   # Your code here
   ```

## Troubleshooting

### "No module named 'kaspawalletd_pb2'"
Run the protoc command to generate Python files first.

### "Failed to connect to remote host"
Check that kaswallet daemon is running and accessible.

### "ModuleNotFoundError: No module named 'grpc'"
Install dependencies: `pip install grpcio grpcio-tools`

## Additional Resources

- [Python gRPC Documentation](https://grpc.io/docs/languages/python/)
- [grpcio Package](https://pypi.org/project/grpcio/)
- [Protocol Buffers Python Tutorial](https://developers.google.com/protocol-buffers/docs/pythontutorial)
