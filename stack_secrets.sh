#!/usr/bin/env bash

# Sourced by integration_test.sh, security_test.sh, performance_test.sh and the gdpr_*_test.sh
# scripts (MAIR-428, MAIR-680).
#
# Each run of a test stack gets its own Ed25519 key pair: Core signs the session JWTs of the stack
# with JWT_PRIVATE_KEY (EdDSA) like in a real deployment. Since mairie360_api_lib 6.0.0 there is no
# JWT_SECRET any more and every HS256 token is refused. The admin JWT the scans need (`sub=1`, the
# Admin seeded by liquibase, `role=admin`, valid 4 hours) is signed here with that key, exactly like
# Core does (`alg` EdDSA, `kid` = RFC 7638 thumbprint of the public key, `iss` mairie360-core, `aud`
# mairie360). Nothing signed with a committed key is left in the repository: the repos are public.
#
# Exports JWT_PRIVATE_KEY / JWT_PUBLIC_KEY (kept when both are already set), TOTP_ENCRYPTION_KEY and
# ADMIN_JWT. Needs openssl 3.

if [ -z "${JWT_PRIVATE_KEY:-}" ] || [ -z "${JWT_PUBLIC_KEY:-}" ]; then
    JWT_PRIVATE_KEY="$(openssl genpkey -algorithm ed25519 2>/dev/null)" || return 1
    JWT_PUBLIC_KEY="$(printf '%s\n' "$JWT_PRIVATE_KEY" | openssl pkey -pubout)" || return 1
fi

# MAIR-805: key sealing the TOTP secrets of the run (base64 of 32 random bytes).
if [ -z "${TOTP_ENCRYPTION_KEY:-}" ]; then
    TOTP_ENCRYPTION_KEY="$(openssl rand -base64 32)" || return 1
fi

b64url() { openssl base64 -A | tr '+/' '-_' | tr -d '='; }

# `kid` of the session tokens: RFC 7638 thumbprint of the Ed25519 public key (the last 32 bytes of
# its DER), computed like mairie360_api_lib does. Derived from the private key that signs, so it
# stays right during a rotation, when JWT_PUBLIC_KEY may list the previous key first.
jwt_kid="$(printf '{"crv":"Ed25519","kty":"OKP","x":"%s"}' \
    "$(openssl pkey -in <(printf '%s\n' "$JWT_PRIVATE_KEY") -pubout -outform DER | tail -c 32 | b64url)" \
    | openssl dgst -sha256 -binary | b64url)" || return 1
jwt_header="$(printf '{"alg":"EdDSA","typ":"JWT","kid":"%s"}' "$jwt_kid" | b64url)"
jwt_payload="$(printf '{"sub":"1","role":"admin","exp":%s,"iss":"mairie360-core","aud":"mairie360"}' \
    "$(( $(date +%s) + 4 * 3600 ))" | b64url)"
# `pkeyutl -rawin` (Ed25519) reads its input from a file, not from a pipe.
jwt_input="$(mktemp)" || return 1
printf '%s.%s' "$jwt_header" "$jwt_payload" >"$jwt_input"
jwt_signature="$(openssl pkeyutl -sign -rawin -in "$jwt_input" -inkey <(printf '%s\n' "$JWT_PRIVATE_KEY") | b64url)"
rm -f "$jwt_input"
[ -n "$jwt_signature" ] || return 1
ADMIN_JWT="$jwt_header.$jwt_payload.$jwt_signature"
unset jwt_kid jwt_header jwt_payload jwt_input jwt_signature

export JWT_PRIVATE_KEY JWT_PUBLIC_KEY TOTP_ENCRYPTION_KEY ADMIN_JWT
