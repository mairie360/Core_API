#!/bin/bash
set -e

# Same folder as the WORKDIR of development.Dockerfile
cd /usr/src/core

# Hot reload of the API (`--bin`: the crate also ships the keycloak_migration binary)
exec cargo watch --poll -w src -i target -x "run --bin core_api"
