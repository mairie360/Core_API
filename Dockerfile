# Define build arguments
ARG NODE_VERSION=23.10.0

# Stage 1: Builder
FROM node:${NODE_VERSION}-bookworm-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/core

# Copy the package files
COPY package.json package-lock.json ./

# Install dependencies
RUN npm ci

# Copy the source code
COPY . .

# Build the project
RUN npm run build

# Create non-root user and group
RUN groupadd --system core && useradd --no-log-init --system -g core core

# Set permissions
USER core

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/npm"]
CMD ["run", "start"]
