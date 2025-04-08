FROM rust:1.75 as builder

WORKDIR /usr/src/icn-covm
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/icn-covm/target/release/icn-covm /usr/local/bin/icn-covm

# Create directory for storage
RUN mkdir -p /app/storage

# Set the entry point to the built binary
ENTRYPOINT ["icn-covm"]
CMD ["--help"] 