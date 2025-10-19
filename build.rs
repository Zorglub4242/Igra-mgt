// Build script to compile Protocol Buffer definitions for kaswallet-daemon gRPC

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/kaspawalletd.proto")?;
    Ok(())
}
