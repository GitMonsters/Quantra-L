fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Future: Add gRPC proto compilation here
    // tonic_build::compile_protos("proto/quantra.proto")?;

    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
