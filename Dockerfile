FROM rust:1.95-bookworm AS builder

WORKDIR /app

# `rusqlite` bundles SQLite and `rustls` builds aws-lc-rs, both of which need a
# C toolchain and cmake.
RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake clang \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked --bins

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/stock-screener /usr/local/bin/stock-screener
COPY --from=builder /app/target/release/update_stocks /usr/local/bin/update_stocks

RUN mkdir -p /app/data && ln -s /app/data /data

EXPOSE 3000

CMD ["stock-screener"]
