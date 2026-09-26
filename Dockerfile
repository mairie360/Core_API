FROM rust:1.98-slim-bookworm AS builder

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
# One-shot job: migrates the accounts and roles to Keycloak (MAIR-141), same env vars as the API.
COPY --from=builder /usr/src/app/target/release/keycloak_migration /app/keycloak-migration

CMD ["/app/core-api"]