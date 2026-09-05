# API runtime image. Do not pass secrets as build args or copy .env files.
FROM rust:1-bookworm AS builder

WORKDIR /src

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY apps/api ./apps/api
COPY apps/worker ./apps/worker
COPY crates ./crates
COPY tests ./tests
COPY migrations ./migrations

RUN cargo build --release --bin market-bot-api

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home-dir /nonexistent --shell /usr/sbin/nologin appuser

COPY --from=builder /src/target/release/market-bot-api /usr/local/bin/market-bot-api

USER appuser

ENV API_BIND_ADDRESS=0.0.0.0:3000 \
    RUST_LOG=info \
    OTEL_SERVICE_NAME=market-bot-api \
    PAYMENT_PROVIDER=sandbox

EXPOSE 3000

HEALTHCHECK --interval=10s --timeout=3s --start-period=20s --retries=5 \
    CMD curl -fsS "http://127.0.0.1:3000/healthz" || exit 1

CMD ["market-bot-api"]
