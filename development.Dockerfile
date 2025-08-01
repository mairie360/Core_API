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
COPY api_macro_lib ./api_macro_lib
COPY api_lib ./api_lib
COPY cargo-watch.toml .

# Définir les variables d’environnement
ENV RUST_BACKTRACE=1
ENV CARGO_HOME=/usr/local/cargo
ENV PATH=$CARGO_HOME/bin:$PATH

# Exposer le port
EXPOSE 3000

# Lancer cargo watch pour recompiler automatiquement en cas de modification
CMD ["cargo", "watch", "-w", "src", "-w", "api_lib", "-w", "api_macro_lib", "-i", "target", "-i", "api_lib/target", "-i", "api_macro_lib/target", "-x", "run"]
