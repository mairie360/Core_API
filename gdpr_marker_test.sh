#!/usr/bin/env bash

# GDPR log marker test (MAIR-290, mairie360/CICD tests/gdpr/marker). Starts
# docker-compose-gdpr-marker.yml, whose `gdpr-marker` runner plays gdpr-marker.yaml with a marker
# user, saves the logs of every container before `down`, then searches them for the marker's
# values. Exit 0: no value found. Exit 1: a value was found, or the journey failed.
#
# The API under test is the image named by IMAGE_REF: in CI the image promoted to staging (by
# digest); when it is empty (local usage) it is built from development.Dockerfile as
# core-api:local. The report (values masked) is written to gdpr-report/summary.md.

COMPOSE_FILE="docker-compose-gdpr-marker.yml"
REPORT_DIR="gdpr-report"

if [ -z "${IMAGE_REF:-}" ]; then
    echo "==> [0/4] IMAGE_REF is empty: building core-api:local from development.Dockerfile..."
    IMAGE_REF="core-api:local"
    docker build -f development.Dockerfile -t "$IMAGE_REF" . || exit 1
fi
export IMAGE_REF
echo "==> API image under test: $IMAGE_REF"

# Random JWT_SECRET for this run and the admin JWT signed with it (MAIR-428).
# shellcheck source=stack_secrets.sh
source ./stack_secrets.sh || exit 1

# Marker engine. CI checks mairie360/CICD out as cicd-repo/; locally it is cloned once at the
# cicd_version pinned in .github/workflows/cicd.yml (override with CICD_VERSION, e.g. a branch).
CICD_DIR="cicd-repo"
if [ ! -f "$CICD_DIR/tests/gdpr/marker/scan.mjs" ]; then
    CICD_VERSION="${CICD_VERSION:-$(sed -n 's/^[[:space:]]*cicd_version:[[:space:]]*\([^[:space:]#]*\).*/\1/p' .github/workflows/cicd.yml | head -n 1)}"
    echo "==> Fetching mairie360/CICD $CICD_VERSION into $CICD_DIR/..."
    rm -rf "$CICD_DIR"
    git clone --quiet --depth 1 --branch "$CICD_VERSION" https://github.com/mairie360/CICD "$CICD_DIR" || exit 1
fi

rm -rf "$REPORT_DIR" && mkdir -p "$REPORT_DIR"
# The stack is always removed, even when the script is interrupted (CI bounds it with `timeout`).
trap 'docker compose -f "$COMPOSE_FILE" down -v >/dev/null 2>&1' EXIT

echo "==> [1/4] Starting the stack and the marker journey..."
docker compose -f "$COMPOSE_FILE" up -d

echo "==> [2/4] Waiting for the journey to finish..."
docker compose -f "$COMPOSE_FILE" wait gdpr-marker

echo "==> [3/4] Saving the logs of every container..."
docker compose -f "$COMPOSE_FILE" logs --no-color > "$REPORT_DIR/containers.log"
docker compose -f "$COMPOSE_FILE" logs --no-color --tail 40 gdpr-marker
docker compose -f "$COMPOSE_FILE" down -v

echo "==> [4/4] Searching the logs for the marker..."
docker run --rm -v "$PWD/$REPORT_DIR:/report" -v "$PWD/$CICD_DIR/tests/gdpr:/engine:ro" \
    node:24-bookworm-slim node /engine/marker/scan.mjs /report
EXIT_CODE=$?

echo "----------------------------------------"
echo "Final exit code: $EXIT_CODE"
echo "----------------------------------------"

exit $EXIT_CODE
