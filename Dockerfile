FROM rust:1.94-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

# --- Étape 2 : Runtime (Ultra-light) ---
FROM gcr.io/distroless/cc-debian12
WORKDIR /app

COPY --from=builder /usr/src/app/target/release/core_api /app/core-api

CMD ["/app/core-api"]