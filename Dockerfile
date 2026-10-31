# Images are pinned by digest (Renovate bumps tag and digest together): a re-pushed tag cannot
# change what gets built. Same toolchain as API_template (MAIR-427). Builder, runtime and
# development.Dockerfile share one Debian release (13 "trixie"), so the binary links against the
# glibc and OpenSSL the runtime image ships (MAIR-741: Debian 12's distroless libssl3 is unpatched).
FROM rust:1.99-slim-trixie@sha256:b8e062b1c3a80183df83d9ba187cadd2b5ab84bc78d453dbfeb2fccd5f982247 AS builder

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
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:3ee00291cf386ec52a552e6a858d555debd74776eb4bfa53fa3df2cd6677515e
WORKDIR /app

COPY --from=builder --chown=0:0 --chmod=0555 /usr/src/app/target/release/core_api /app/core-api
# One-shot job: migrates the accounts and roles to Keycloak (MAIR-141), same env vars as the API.
COPY --from=builder --chown=0:0 --chmod=0555 /usr/src/app/target/release/keycloak_migration /app/keycloak-migration

# Numeric uid/gid, so Kubernetes `runAsNonRoot: true` can verify it without resolving a name.
USER 65532:65532

CMD ["/app/core-api"]
