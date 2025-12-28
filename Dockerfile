# build stage
FROM rust:1.92 as builder

WORKDIR /etc/cfddns

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY src ./src
RUN touch src/main.rs
RUN cargo build --release

# runtime
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /etc/cfddns

COPY --from=builder /etc/cfddns/target/release/cfddns /etc/cfddns/cfddns

ENTRYPOINT [ "/etc/cfddns/cfddns" ]
