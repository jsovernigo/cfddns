# build stage
FROM rust:1.75 as builder

WORKDIR /etc/cfddns

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

COPY src ./src
RUN cargo build --release

# runtime
from debian:bookworm-slim

WORKDIR /etc/cfddns

COPY --from=builder /etc/cfddns/target/release/cfddns /etc/cfddns/cfddns
COPY .env /etc/cfddns/.env

ENTRYPOINT [ "/etc/cfddns/cfddns" ]
