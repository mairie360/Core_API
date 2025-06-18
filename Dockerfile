# Étape de construction et d'exécution
FROM rust:latest AS development

# Installer les dépendances nécessaires
RUN apt update && apt install -y --no-install-recommends \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Définir le répertoire de travail
WORKDIR /usr/src/core

# Copier les fichiers de configuration et le code source
COPY Cargo.toml ./
COPY src ./src

# Créer un utilisateur non-root
RUN useradd --system --home /usr/src/core --shell /usr/sbin/nologin core

# Définir les permissions
RUN chown -R core:core /usr/src/core
USER core

# Définir les variables d'environnement
ENV RUST_BACKTRACE=1
ENV HOSTNAME="0.0.0.0"
ENV PORT=3000

# Exposer le port
EXPOSE 3000

# Commande pour exécuter le projet en mode release
CMD ["cargo", "run", "--release"]
