#!/usr/bin/env bash

# Runs the OWASP ZAP scan (docker-compose-security.yml) against the API. The exit code of the
# `security-scan` service is the exit code of this script: any WARN/FAIL alert fails CI.
#
# The API under test is the image named by IMAGE_REF. CI sets it to the published
# ghcr.io/mairie360/core-api:dev-<sha> image (the one promoted to staging and prod); when it is
# empty (local usage) the image is built from development.Dockerfile as core-api:local.

COMPOSE_FILE="docker-compose-security.yml"
SERVICE_NAME="security-scan"

if [ -z "${IMAGE_REF:-}" ]; then
    echo "==> [0/4] IMAGE_REF is empty: building core-api:local from development.Dockerfile..."
    IMAGE_REF="core-api:local"
    docker build -f development.Dockerfile -t "$IMAGE_REF" . || exit 1
fi
export IMAGE_REF
echo "==> API image under test: $IMAGE_REF"

echo "==> [1/4] Starting the stack and the ZAP scan..."
docker compose -f "$COMPOSE_FILE" up -d

echo "==> [2/4] Waiting for the ZAP scan to finish..."
docker compose -f "$COMPOSE_FILE" wait "$SERVICE_NAME"
EXIT_CODE=$?

echo "==> [3/4] Report (logs)..."
docker compose -f "$COMPOSE_FILE" logs "$SERVICE_NAME"

echo "==> [4/4] Cleaning up the containers..."
docker compose -f "$COMPOSE_FILE" down

echo "----------------------------------------"
echo "Final exit code: $EXIT_CODE"
echo "----------------------------------------"

exit $EXIT_CODE
