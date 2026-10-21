# Same toolchain as the production `Dockerfile`, pinned by digest (MAIR-427).
FROM rust:1.99-bookworm@sha256:5c1a1732ac3f77635b24d397ceb67b4d9ea61de348f68c0aa54a79fd01e65c69 AS development

RUN apt update && apt install -y curl && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-watch --locked

# Must match the `develop.watch` targets of docker-compose.yml and entrypoint.sh
WORKDIR /usr/src/core

# --- DEPENDENCY CACHE ---
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin \
    && echo "fn main() {}" > src/main.rs \
    && echo "fn main() {}" > src/bin/keycloak_migration.rs \
    && touch src/lib.rs
# This layer stays cached as long as Cargo.toml and Cargo.lock do not change
RUN cargo build --locked && rm -rf src
# -----------------------------

COPY src ./src
# The placeholder sources of the dependency cache are newer than the real ones: without this
# touch, cargo keeps the empty `fn main() {}` binary and the container never serves (same as the
# production Dockerfile).
RUN touch src/main.rs src/lib.rs src/bin/keycloak_migration.rs
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

EXPOSE 3000
# Development image only (hot reload, bind mounts): it runs as root on purpose.
# nosemgrep: dockerfile.security.missing-user.missing-user
CMD ["/usr/local/bin/entrypoint.sh"]