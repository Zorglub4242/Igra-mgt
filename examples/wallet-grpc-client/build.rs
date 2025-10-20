/// Build script to compile protobuf definitions to Rust code
/// This runs at compile time before the main build

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure tonic-build to generate gRPC client code from proto files
    tonic_build::configure()
        .build_server(false) // We only need the client, not the server
        .compile(
            &["proto/kaspawalletd.proto"], // Proto file to compile
            &["proto"],                     // Include path for proto imports
        )?;

    println!("cargo:rerun-if-changed=proto/kaspawalletd.proto");

    Ok(())
}
