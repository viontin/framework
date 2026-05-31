# ──── Build Stage ────
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
RUN cargo build --release --features shutdown

# ──── Runtime Stage ────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/your-app /app/app
COPY config/ /app/config/
COPY public/ /app/public/
COPY storage/ /app/storage/

EXPOSE 3000

ENV APP_ENV=production
ENV APP_DEBUG=false

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/healthz || exit 1

CMD ["./app"]
