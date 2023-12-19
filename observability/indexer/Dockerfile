FROM rust:1.72 as builder

RUN apt-get update -y && apt-get install -y pkg-config build-essential libudev-dev clang cmake protobuf-compiler
RUN rustup component add rustfmt clippy

WORKDIR /app

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./programs ./programs
COPY ./observability/indexer ./observability/indexer
COPY ./test-utils ./test-utils
COPY ./tools ./tools
COPY ./clients/rust ./clients/rust

ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

RUN cargo build --release


FROM debian:stable-slim as runner

WORKDIR /app

RUN apt-get update -y && apt-get install -y ca-certificates openssl

COPY --from=builder /app/target/release/mfi-index ./

ENV RUST_LOG=info

CMD ["/app/mfi-index"]
