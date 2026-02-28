# development.Dockerfile
FROM rust:latest AS development

# Installer les dépendances nécessaires
RUN apt update && apt install -y --no-install-recommends \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Installer cargo-watch pour le hot-reload
RUN cargo install cargo-watch

# Définir le répertoire de travail
WORKDIR /usr/src/core

# Copier uniquement les fichiers nécessaires
COPY Cargo.toml .
COPY Cargo.lock .
COPY src ./src
COPY api_lib ./api_lib

# Définir les variables d’environnement
ENV RUST_BACKTRACE=1
ENV CARGO_HOME=/usr/local/cargo
ENV PATH=$CARGO_HOME/bin:$PATH

# Exposer le port
EXPOSE 3000

# Copier le script d'entrée et le rendre exécutable
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Lancer cargo watch pour recompiler automatiquement en cas de modification
CMD ["/usr/local/bin/entrypoint.sh"]