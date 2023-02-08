fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .type_attribute("PubsubTransaction", "#[derive(serde::Serialize)]")
        .compile(
            &[
                "protos/geyser.proto",
                "protos/solana_storage.proto",
                "protos/gcp_pubsub.proto",
            ],
            &["protos"],
        )
        .unwrap();
    Ok(())
}
