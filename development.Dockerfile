# Utiliser une image officielle Rust comme base
FROM rust:latest AS development

# Installer les dépendances nécessaires
RUN apt update && apt install -y --no-install-recommends \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Installer cargo-watch
RUN cargo install cargo-watch

# Créer un utilisateur non-root avec un shell valide
RUN useradd --create-home --shell /bin/bash core

# Définir le répertoire de travail et les permissions
WORKDIR /usr/src/core
RUN chown core:core /usr/src/core

# Définir les permissions pour le cache de Cargo
RUN mkdir -p /usr/local/cargo/registry && \
    chown -R core:core /usr/local/cargo

# Basculer vers l'utilisateur non-root
USER core

# Copier le fichier de configuration de Rust et le code source
COPY --chown=core:core Cargo.toml ./
COPY --chown=core:core src ./src

# Définir les variables d'environnement
ENV RUST_BACKTRACE=1
ENV HOSTNAME="0.0.0.0"
ENV PORT=3000
ENV CARGO_HOME=/usr/local/cargo

# Exposer le port
EXPOSE 3000

# Commande pour exécuter le projet avec cargo watch
CMD ["cargo", "watch", "-x", "run"]
