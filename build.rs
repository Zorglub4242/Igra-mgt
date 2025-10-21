// Build script to compile Protocol Buffer definitions for kaswallet-daemon gRPC
// and capture build timestamp

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile protobuf definitions
    tonic_build::compile_protos("proto/kaspawalletd.proto")?;

    // Capture build timestamp
    let build_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S %Z").to_string();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_time);

    // Note: Not using rerun-if-changed means this script runs on every build,
    // ensuring BUILD_TIMESTAMP is always current

    Ok(())
}
