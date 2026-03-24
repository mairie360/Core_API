FROM rust:latest AS development

RUN apt update && apt install -y curl && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-watch

# Utilisons un chemin simple et unique
WORKDIR /usr/src/core

# --- CACHE DES DÉPENDANCES ---
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
# Cette couche sera mise en cache tant que Cargo.toml ne change pas
RUN cargo build && rm -rf src 
# -----------------------------

COPY src ./src
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

EXPOSE 3000
CMD ["/usr/local/bin/entrypoint.sh"]