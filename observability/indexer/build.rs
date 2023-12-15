fn main() -> anyhow::Result<()> {
    compile_protos()?;
    Ok(())
}

fn compile_protos() -> anyhow::Result<()> {
    std::env::set_var("PROTOC", protobuf_src::protoc());
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .type_attribute("PubsubTransaction", "#[derive(serde::Serialize)]")
        .type_attribute("PubsubAccountUpdate", "#[derive(serde::Serialize)]")
        .compile(
            &["protos/geyser.proto", "protos/gcp_pubsub.proto"],
            &["protos"],
        )
        .unwrap();
    Ok(())
}
