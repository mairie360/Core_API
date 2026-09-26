# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Core API of the **mairie360** platform: an Actix-web REST API (Rust, edition 2021) providing users, sessions,
roles, permissions, groups and resource-access management. It's designed to run behind other "module" services
that call into it for auth/authz.

Most cross-cutting infrastructure (DB pool, Redis helpers, JWT/session middleware, env var loading, test
scaffolding) lives in the external crate `mairie360_api_lib`, whose source is the sibling repo `../API_lib`. When
a type/function isn't defined in this repo, look there.

## Commands

Aliases are defined in `.cargo/config.toml`:

```bash
cargo lint_check   # fmt --all -- --check
cargo lint_fix     # fmt --all
cargo check_code   # clippy --all-targets --all-features -- -D warnings
cargo open_api     # runs examples/generate_openapi.rs, prints the OpenAPI JSON (redirect to openapi.json)
```

Build/run:

```bash
cargo build
cargo build --release
docker compose up --build --watch   # full stack: postgres, liquibase migrations, redis, core (hot reload), mailpit, nginx
```

The dev stack is reached via nginx at `http://development.mairie360.fr`. `core` requires `HOST`, `PORT`,
`REDIS_URL`, `DB_USER`, `DB_PASSWORD`, `DB_HOST`, `DB_PORT`, `DB_NAME`, `JWT_SECRET`, `JWT_TIMEOUT` and
`SMTP_*`/`EMAIL_FROM` env vars — all fetched with `get_critical_env_var` (panics on missing var), see
`docker-compose.yml` for the dev values. The Postgres URL is assembled by
`database::pg_url::build_pg_url`, which percent-encodes user, password and database name, so
`DB_PASSWORD` may contain any character.

Keycloak sign-in (`POST /api/v1/auth/keycloak`, `src/keycloak/`) is optional and read with
`get_env_var`: `KEYCLOAK_REALM_URL` + `KEYCLOAK_CLIENT_ID` enable it, `KEYCLOAK_CLIENT_SECRET`
(confidential client) and `KEYCLOAK_ISSUER` (public issuer when Core reaches Keycloak through an
internal URL) are optional. Without them the route answers `503` and only the password login
works. Core redeems the authorization code, verifies the ID token against the realm JWKS and opens
the same Core session (JWT + refresh token) as a password login. The account is resolved by the
Keycloak user id (`sub`) recorded in `user_identities` (`resolve_user_identity()`), falling back to
the token's verified e-mail for an account not linked yet, which links it on the spot
(`link_user_identity()`); no account is created and a subject linked to another account is
refused. Tests use a local fake realm (`tests/common/keycloak_mock.rs`, throwaway RSA keys in
`tests/fixtures/`), no Keycloak needed.

Account migration to Keycloak (MAIR-141): `POST /api/v1/admin/keycloak/migration` (admin only)
and the `keycloak_migration` binary (`src/bin/`, shipped in the image as `/app/keycloak-migration`,
same env vars, `--send-password-setup-email` flag) both run `keycloak::migration::migrate_users`:
read `v_users_sso_export`, find or create each account in the realm through the Admin REST API
(`src/keycloak/admin.rs`, service account of the confidential client, which needs the
`realm-management` roles `manage-users`, `view-realm`, `manage-realm`), map Core roles as realm
roles, disable archived accounts, then link through `link_user_identity()`. Replayable without
duplicates. Requires the `1.4.0` schema (`user_identities`, `users.password` nullable): the
compose files pin it and `.cargo/config.toml` sets `TEST_DB_VERSION` for the testcontainers.

Administration mirroring (MAIR-142, `src/keycloak/sync.rs`): when `main.rs` registers a
`KeycloakAdminClient` as app data (confidential client only), `POST /api/v1/admin/users/`,
`PATCH`/`DELETE /api/v1/admin/users/{id}/` and the role grant/revoke take it as
`Option<web::Data<KeycloakAdminClient>>` and mirror the change into the realm **before** the
Core write, compensating the Keycloak side (delete the reserved account, restore the former
profile, re-enable, re-map...) when Core refuses. Accounts are found by the recorded link, then
by e-mail (linked on the spot); an account unknown to Keycloak is left to the migration, and
without the client only Core is written. `DELETE /api/v1/admin/users/{id}/` archives through
`DELETE FROM v_users_active` (the schema has no `delete_user()` function; the endpoint was
broken before MAIR-142). The fake realm of `tests/common/keycloak_mock.rs` covers the Admin API
subset used; `tests/endpoints/admin_users_keycloak.rs` exercises every path.

Tests:

```bash
cargo test                                   # all tests
cargo test --test integration_test queries::auth::login::test_login_user_success
```

End-to-end tests (what CI runs on `main` after the dev release, needs Docker + GHCR pull access):

```bash
./integration_test.sh    # docker-compose-integration.yml: full stack + newman replaying tests/postman/collection.json
./security_test.sh       # docker-compose-security.yml: full stack + ZAP scan of /api-docs/openapi.json
./performance_test.sh    # docker-compose-performance.yml: full stack + k6 (load-test.js)
```

`tests/postman/collection.json` is a Postman v2.1 collection (importable in the app) and
`tests/postman/environment.json` its variables; the compose file overrides `baseUrl` with `--env-var` so the
committed default (`http://localhost:3000`) stays usable from a host shell. The scenario registers a fresh user
(unique e-mail generated in the collection pre-request script) so it is replayable against a persistent database.

`tests/routing_test.rs` checks that every `/api/v1` operation published by `ApiDoc` (the contract
`@mairie360/core-api-openapi` is generated from) hits a mounted actix route: Core does not normalise trailing
slashes, and utoipa replaces (does not merge) two `nest` entries that end on the same path, so operations sharing
a path must be merged into one document first (see `admin/users/doc.rs`).

Integration tests need **Docker running** — `get_shared_db()` (from `mairie360_api_lib::test_setup`) spins up a
shared Postgres testcontainer on first use and hands back a pool. Tests that touch shared/seeded rows are
annotated `#[serial]` (the `serial_test` crate) to avoid interference between tests running against the same
container.

OpenAPI client generation (consumed by other repos, not by this API itself):

```bash
npm install
npx orval   # reads openapi.json, writes generated/ (per orval.config.js)
```

## Architecture

### Two parallel trees: `database/` and `endpoints/v1/`

Business logic is split into two mirrored trees under `src/`, one per resource/action:

- `src/database/<domain>/<action>/` — the DB access layer. Each folder is a triad:
  - `mod.rs` — re-exports the query fn and view types
  - `query.rs` — an async fn taking a `*QueryView` + `sqlx::PgPool`, returning
    `Result<T, mairie360_api_lib::database::errors::DatabaseError>`
  - `view.rs` — a `*QueryView` struct implementing `mairie360_api_lib::database::db_interface::DatabaseQueryView`
    (its `get_request()` returns the raw SQL string with `$n` placeholders bound positionally by the query fn),
    plus a `*QueryResultView` (`#[derive(sqlx::FromRow)]`) for the row shape.

- `src/endpoints/v1/<domain>/<action>/` — the HTTP layer, same triad idea:
  - `endpoint.rs` — the actix handler (`#[get]`/`#[post]`/etc + `#[utoipa::path]`), an `*Error` enum
    implementing `ResponseError` for domain-specific failure→status mapping, and the handler logic that calls
    into `database::` query functions via `state.db_pool.clone().unwrap()` where `state: web::Data<AppState>`
  - `view.rs` — request/response DTOs (`serde` + `utoipa::ToSchema`)
  - `doc.rs` — a `#[derive(OpenApi)]` struct listing that endpoint's paths/schemas, aggregated upward into a
    per-domain doc, ultimately into `endpoints::swagger::ApiDoc`

When adding a new endpoint, follow an existing sibling (e.g. `src/endpoints/v1/auth/login/` +
`src/database/auth/login/`) rather than inventing a new shape.

### Routing

`endpoints::config()` mounts `health`/`hello`/swagger-ui unauthenticated, and `v1::config()`. In `main.rs`, the
whole `/api` scope is wrapped in `mairie360_api_lib::security::JwtMiddleware`; within `v1::config()`, the
`admin::config()` scope is additionally wrapped in `AdminMiddleware`. Each domain module (`auth`, `groups`,
`roles`, `sessions`, `user`, `admin`, `ressources`, ...) exposes its own `config(cfg: &mut ServiceConfig)` that
nests further scopes — follow the chain from `main.rs` → `endpoints/mod.rs` → `endpoints/v1/mod.rs` → domain
`mod.rs` to see the full route tree for a path.

### State, DB and Redis

`AppState` (from `mairie360_api_lib::pool`) is built once in `main.rs` from `REDIS_URL` + a constructed Postgres
URL and shared via `web::Data`. It exposes `db_pool` (a `sqlx::PgPool`, wrapped for shared access — note direct
`sqlx::PgPool` usage in query fns) and Redis access via `state.get_redis_conn()`. Simple encrypted key/value ops
against Redis go through `mairie360_api_lib::pool::redis::simple_key::secured::{handle_secure_get, handle_secure_post}`
(used e.g. for the one-time first-login token, see `endpoints/v1/auth/login/endpoint.rs`).

Note: `Cargo.toml` has no direct `sqlx` dependency — it's pulled in transitively through
`mairie360_api_lib` (1.2.2), which is why files can `use sqlx::...` without it being listed directly. A
leftover direct `tokio-postgres` dependency also still exists in `Cargo.toml`; the DB/Redis management story is
mid-refactor (see current branch), so don't be surprised if both appear for a while.

### Sessions / auth flow

Login (`endpoints/v1/auth/login/endpoint.rs`) checks credentials, and on a user's first connection returns
`412 Precondition Failed` with a one-time token (stored in Redis) instead of logging in, forcing a password
change via `force_change_password`. On success it issues a JWT (`mairie360_api_lib::jwt_manager::generate_jwt`)
plus an opaque refresh token persisted as a session row (`database::sessions::create_session`).

### Database schema

Postgres schema is **not** managed in this repo — it lives in a separate Liquibase migrations image
(`ghcr.io/mairie360/liquibase-migrations`, run as the `liquibase` service in `docker-compose.yml`) applied
against the `ghcr.io/mairie360/database` image. `DATABASE.md` is a (partial/stale) reference sketch of the
schema, not the source of truth.

### Docs referenced in this repo

- `API.md` — informal endpoint list (partial/stale in places, e.g. missing rows).
- `DATABASE.md` — schema sketch, see caveat above.

## CI

`.github/workflows/cicd.yml` delegates to the reusable `mairie360/CICD` workflow (`APIs_cicd.yml`) on every
push; the `integration_tests` job runs `./integration_test.sh` (newman, no Postman account involved). `auto-approve.yml` auto-approves Renovate
PRs.

## Pull request reviewers

Every PR requests a review from the whole team, minus its author: `CarolinHugo`, `LAURETbenjamin`, `MathTek` and `Quentintnrl` (`gh pr create … --reviewer CarolinHugo,LAURETbenjamin,MathTek`). `.github/CODEOWNERS` makes GitHub request them automatically as well.
