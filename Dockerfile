# build stage
FROM rust:1.92 as builder

WORKDIR /etc/cfddns

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

COPY src ./src
RUN cargo build --release

# runtime
FROM debian:bookworm-slim

WORKDIR /etc/cfddns

COPY --from=builder /etc/cfddns/target/release/cfddns /etc/cfddns/cfddns

ENTRYPOINT [ "/etc/cfddns/cfddns" ]
