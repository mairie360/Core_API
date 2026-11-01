#!/usr/bin/env bash

# Sourced by integration_test.sh, security_test.sh and performance_test.sh (MAIR-428).
#
# Each run of a test stack gets its own random JWT_SECRET, and the admin JWT the scans need
# (`sub=1`, the Admin seeded by liquibase, `role=admin`, valid 4 hours) is signed with it here,
# exactly like mairie360_api_lib::jwt_manager does (HS256, claims { sub, role, exp }). Nothing
# signed with a committed secret is left in the repository: the repos are public, so a committed
# token would be an admin token anywhere that secret was reused.
#
# MAIR-680: the run also gets its own Ed25519 key pair, so Core signs the session JWTs of the
# stack with JWT_PRIVATE_KEY (EdDSA) like in a real deployment; JWT_SECRET stays set for the
# transition (ADMIN_JWT is still an HS256 token, accepted while the secret is set).
#
# Exports JWT_SECRET (kept when already set), JWT_PRIVATE_KEY / JWT_PUBLIC_KEY (kept when both are
# already set) and ADMIN_JWT. Needs openssl.

if [ -z "${JWT_SECRET:-}" ]; then
    JWT_SECRET="$(openssl rand -hex 32)" || return 1
fi

if [ -z "${JWT_PRIVATE_KEY:-}" ] || [ -z "${JWT_PUBLIC_KEY:-}" ]; then
    JWT_PRIVATE_KEY="$(openssl genpkey -algorithm ed25519 2>/dev/null)" || return 1
    JWT_PUBLIC_KEY="$(printf '%s\n' "$JWT_PRIVATE_KEY" | openssl pkey -pubout)" || return 1
fi

b64url() { openssl base64 -A | tr '+/' '-_' | tr -d '='; }

jwt_header="$(printf '%s' '{"alg":"HS256","typ":"JWT"}' | b64url)"
jwt_payload="$(printf '{"sub":"1","role":"admin","exp":%s}' "$(( $(date +%s) + 4 * 3600 ))" | b64url)"
jwt_signature="$(printf '%s.%s' "$jwt_header" "$jwt_payload" \
    | openssl dgst -sha256 -hmac "$JWT_SECRET" -binary | b64url)"
ADMIN_JWT="$jwt_header.$jwt_payload.$jwt_signature"
unset jwt_header jwt_payload jwt_signature

export JWT_SECRET JWT_PRIVATE_KEY JWT_PUBLIC_KEY ADMIN_JWT
