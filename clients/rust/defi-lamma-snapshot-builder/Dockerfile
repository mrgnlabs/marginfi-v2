FROM rust:1.66 as builder

RUN apt-get update -y && apt-get install -y pkg-config build-essential libudev-dev clang cmake protobuf-compiler
RUN rustup component add rustfmt clippy

WORKDIR /app

COPY ./Cargo.toml ./Cargo.toml
COPY ./programs ./programs
COPY ./clients/rust ./clients/rust
COPY ./test-utils ./test-utils

RUN cargo update
RUN cargo build --release

FROM debian:stable-slim as runner

WORKDIR /app

RUN apt-get update -y && apt-get install -y ca-certificates

COPY --from=builder /app/target/release/llama-snapshot ./

# ENV RUST_LOG=info

CMD ["/app/llama-snapshot"]
