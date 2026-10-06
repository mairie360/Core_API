# Images are pinned by digest (Renovate bumps tag and digest together): a re-pushed tag cannot
# change what gets built. Same toolchain as API_template (MAIR-427).
FROM rust:1.99-slim-bookworm@sha256:2c3a22f0a5533ea2dd5a16627bc841228151faa2d4de2644ac9987e4a2f1f2fa AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# --- Dependency cache: rebuilt only when Cargo.toml or Cargo.lock change ---
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin \
    && echo "fn main() {}" > src/main.rs \
    && echo "fn main() {}" > src/bin/keycloak_migration.rs \
    && touch src/lib.rs \
    && cargo build --release --locked \
    && rm -rf src

COPY . .
# `--locked`: build exactly the reviewed `Cargo.lock`, fail instead of resolving new versions.
# `touch`: the placeholder sources above are newer than the real ones' checkout time.
RUN touch src/main.rs src/lib.rs src/bin/keycloak_migration.rs \
    && cargo build --release --locked

# --- Stage 2: runtime (distroless, non-root) ---
# `:nonroot` runs as uid/gid 65532. The binary stays owned by root and is only readable and
# executable by that user: the API never writes to the filesystem (configuration comes from
# environment variables, state lives in Postgres and Redis), so a read-only root filesystem works.
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f
WORKDIR /app

COPY --from=builder --chown=0:0 --chmod=0555 /usr/src/app/target/release/core_api /app/core-api
# One-shot job: migrates the accounts and roles to Keycloak (MAIR-141), same env vars as the API.
COPY --from=builder --chown=0:0 --chmod=0555 /usr/src/app/target/release/keycloak_migration /app/keycloak-migration

# Numeric uid/gid, so Kubernetes `runAsNonRoot: true` can verify it without resolving a name.
USER 65532:65532

CMD ["/app/core-api"]
