FROM rust:1.98-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

# --- Stage 2: runtime (distroless, non-root) ---
# `:nonroot` runs as uid/gid 65532. The binary stays owned by root and is only readable and
# executable by that user: the API never writes to the filesystem (configuration comes from
# environment variables, state lives in Postgres and Redis), so a read-only root filesystem works.
FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /app

COPY --from=builder --chown=0:0 --chmod=0555 /usr/src/app/target/release/core_api /app/core-api

# Numeric uid/gid, so Kubernetes `runAsNonRoot: true` can verify it without resolving a name.
USER 65532:65532

CMD ["/app/core-api"]