FROM rust:latest AS builder

WORKDIR /usr/src/abralang

COPY . .

RUN cargo build --release && cargo test --no-run --release

RUN cp target/release/abra /usr/local/bin/abra

ENTRYPOINT ["/bin/bash","-c","timeout --foreground 30s cargo test --release -- --nocapture"]
