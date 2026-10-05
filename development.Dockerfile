# Same toolchain as the production `Dockerfile`, pinned by digest (MAIR-427).
FROM rust:1.99-bookworm@sha256:59037199c44290f2befcdd58dcc540164763fc296950255aaefeef096a1866b0 AS development

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
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

EXPOSE 3000
# Development image only (hot reload, bind mounts): it runs as root on purpose.
# nosemgrep: dockerfile.security.missing-user.missing-user
CMD ["/usr/local/bin/entrypoint.sh"]