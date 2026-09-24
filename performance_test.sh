#!/usr/bin/env bash

# Runs the k6 load test (docker-compose-performance.yml) against the API. The exit code of the
# `k6-perf-test` service is the exit code of this script: a crossed threshold fails CI.
#
# The API under test is the image named by IMAGE_REF. CI sets it to the published
# ghcr.io/mairie360/core-api:dev-<sha> image (the one promoted to staging and prod); when it is
# empty (local usage) the image is built from development.Dockerfile as core-api:local.

COMPOSE_FILE="docker-compose-performance.yml"
SERVICE_NAME="k6-perf-test"

if [ -z "${IMAGE_REF:-}" ]; then
    echo "==> [0/4] IMAGE_REF is empty: building core-api:local from development.Dockerfile..."
    IMAGE_REF="core-api:local"
    docker build -f development.Dockerfile -t "$IMAGE_REF" . || exit 1
fi
export IMAGE_REF
echo "==> API image under test: $IMAGE_REF"

echo "==> [1/4] Starting the stack and the k6 load test..."
docker compose -f "$COMPOSE_FILE" up -d

echo "==> [2/4] Waiting for the k6 load test to finish..."
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
