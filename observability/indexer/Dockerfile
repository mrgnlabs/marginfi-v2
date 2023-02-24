FROM rust:1.66 as builder

RUN apt-get update -y && apt-get install -y pkg-config build-essential libudev-dev clang cmake protobuf-compiler
RUN rustup component add rustfmt clippy

WORKDIR /app

COPY . .

RUN cargo build --release


FROM debian:stable-slim as runner

WORKDIR /app

COPY --from=builder /app/target/release/mfi-index ./

ENV RUST_LOG=info
