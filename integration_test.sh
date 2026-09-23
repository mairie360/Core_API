#!/usr/bin/env bash

# Runs the Postman collection committed under tests/postman/ with newman, against the API built
# from this checkout (docker-compose-integration.yml). Same shape as security_test.sh: the exit
# code of the `newman` service is the exit code of this script, so CI fails on the first failing
# request (`--bail`). No Postman account or API key is needed.

COMPOSE_FILE="docker-compose-integration.yml"
SERVICE_NAME="newman"

echo "==> [1/4] Starting the stack and running the newman collection..."
docker compose -f "$COMPOSE_FILE" up -d --build

echo "==> [2/4] Waiting for the integration tests to finish..."
docker compose -f "$COMPOSE_FILE" wait "$SERVICE_NAME"
EXIT_CODE=$?

echo "==> [3/4] Test report (logs)..."
docker compose -f "$COMPOSE_FILE" logs "$SERVICE_NAME"

echo "==> [4/4] Cleaning up the containers..."
docker compose -f "$COMPOSE_FILE" down

echo "----------------------------------------"
echo "Final exit code: $EXIT_CODE"
echo "----------------------------------------"

exit $EXIT_CODE
