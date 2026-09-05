# Worker runtime image. Do not pass secrets as build args or copy .env files.
FROM rust:1-bookworm AS builder

WORKDIR /src

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY apps/api ./apps/api
COPY apps/worker ./apps/worker
COPY crates ./crates
COPY tests ./tests
COPY migrations ./migrations

RUN cargo build --release --bin market-bot-worker

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home-dir /nonexistent --shell /usr/sbin/nologin appuser

COPY --from=builder /src/target/release/market-bot-worker /usr/local/bin/market-bot-worker

USER appuser

ENV RUST_LOG=info \
    OTEL_SERVICE_NAME=market-bot-worker \
    PAYMENT_PROVIDER=sandbox \
    LOGISTICS_PROVIDER=sandbox

CMD ["market-bot-worker"]
