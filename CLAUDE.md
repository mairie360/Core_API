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
`docker-compose.yml` for the dev values.

Tests:

```bash
cargo test                                   # all tests
cargo test --test integration_test queries::auth::login::test_login_user_success
```

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
`mairie360_api_lib` (>= 1.1.0), which is why files can `use sqlx::...` without it being listed directly. A
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
push; it wires in the Postman collection/environment IDs for this API. `auto-approve.yml` auto-approves Renovate
PRs.
