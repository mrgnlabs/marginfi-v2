fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .type_attribute("PubsubTransaction", "#[derive(serde::Serialize)]")
        .type_attribute("PubsubAccountUpdate", "#[derive(serde::Serialize)]")
        .compile(
            &[
                "protos/geyser.proto",
                "protos/solana-storage-v1.10.40.proto",
                "protos/gcp_pubsub.proto",
            ],
            &["protos"],
        )
        .unwrap();
    Ok(())
}
