FROM rust:1.66 as builder

RUN apt-get update -y && apt-get install -y pkg-config build-essential libudev-dev clang cmake protobuf-compiler
RUN rustup component add rustfmt clippy

WORKDIR /app

COPY . .

WORKDIR /app/observability/indexer

RUN cargo build --release


FROM debian:stable-slim as runner

WORKDIR /app

COPY --from=builder /app/observability/indexer/target/release/mfi-index ./

ENV RUST_LOG=info

CMD ["/app/mfi-index"]
