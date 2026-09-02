# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`core_api` is the **Core** microservice of the mairie360 platform (Rust / actix-web). It owns authentication (login, register, password reset, JWT + refresh sessions), users, roles/permissions (RBAC), groups, and resource-level access grants. It's one service among several (see the sibling `mairie360_api_lib` crate and the `liquibase-migrations` / `database` / `CICD` images referenced in `docker-compose.yml`) — DB schema migrations are **not** in this repo, they live in the `ghcr.io/mairie360/liquibase-migrations` image.

`API.md` and `DATABASE.md` are hand-written and stale/incomplete (truncated table defs, garbled lines) — don't treat them as authoritative. The real API surface is the code under `src/endpoints` plus the generated `openapi.json`.

## Commands

Dev environment is Docker-first:

```bash
docker compose up --build --watch   # full stack: postgres, liquibase migrations, redis, core (hot reload), mailpit, nginx
```

Cargo aliases (defined in `.cargo/config.toml`):

```bash
cargo lint_check    # cargo fmt --all -- --check
cargo lint_fix       # cargo fmt --all
cargo check_code     # cargo clippy --all-targets --all-features -- -D warnings
cargo open_api       # runs examples/generate_openapi.rs, prints ApiDoc as JSON (redirect to openapi.json)
```

Tests:

```bash
cargo test                              # all tests
cargo test test_login_user_success      # a single test by name
cargo test --test integration_test      # the integration test binary (tests/)
```

Integration tests live under `tests/` and call the `database::*` query layer directly against a **shared Postgres testcontainer** (`mairie360_api_lib::test_setup::queries_setup::get_shared_db`), not against the HTTP layer. This means:
- Docker must be running to execute `cargo test`.
- Tests that touch shared/mutating state (e.g. role counts in `tests/common/roles.rs`) are annotated `#[serial]` (from `serial_test`) because they share one DB container — don't strip `#[serial]` when adding tests that mutate shared rows.

TypeScript client generation (consumers of this API):

```bash
npx orval   # reads openapi.json (orval.config.js), writes generated/endpoints + generated/model (axios client, split by tag)
```
Regenerate `openapi.json` via `cargo open_api` first if endpoints changed.

## Règles de gestion des dépendances
- **Interdiction de modifier les dépendances** : Ne pas ajouter, supprimer ou modifier les versions des dépendances dans `Cargo.toml`, `Cargo.lock`, `package.json`, etc. Aucune décision ou modification ne doit être portée sur cette partie sans instruction explicite.

## Cycle de validation
Le cycle de validation obligatoire à suivre est le suivant :
1. `check deps`
2. `check lint`
3. `check build`
4. `check build openapi`
5. `check run orval`
6. `check test unitaire`

## Architecture

### Layering: endpoints → database

Every feature is split across two parallel trees that mirror each other's domain structure:

- **`src/endpoints/v1/<domain>/<action>/`** — the HTTP layer. Each action directory follows a fixed shape:
  - `endpoint.rs` — the actix handler (`#[post(...)]`/`#[get(...)]` etc.), request/error handling, `#[utoipa::path(...)]` annotation.
  - `view.rs` — request/response DTOs (`Serialize`/`Deserialize`/`ToSchema`), never the raw DB row types.
  - `doc.rs` — a `#[derive(OpenApi)]` struct nesting that one handler + its schemas, so docs compose bottom-up.
  - `mod.rs` — re-exports (`pub mod doc; pub mod endpoint; pub mod view;`).
- **`src/database/<domain>/<action>/`** — the data layer, same nesting convention:
  - `query.rs` — the actual `sqlx::query_as` call, executed against a `PgPool`.
  - `view.rs` — a `*QueryView` (implements `mairie360_api_lib::database::db_interface::DatabaseQueryView::get_request() -> String` to supply the SQL) and a `*QueryResultView` (`#[derive(sqlx::FromRow)]`).
  - `mod.rs` — re-exports.

Handlers never write SQL inline — they build a `*QueryView`, call the corresponding `*_query()` function with a pool from `AppState`, and map `DatabaseError` into a domain-specific `*Error` enum that implements `actix_web::ResponseError` (see `LoginError` in `endpoints/v1/auth/login/endpoint.rs` for the pattern: `Display` for the message, `ResponseError::status_code`/`error_response` for the HTTP mapping).

Note: the resource-and-permissions domain is spelled `ressources` (French) throughout both trees (`src/endpoints/v1/ressources`, `src/database/ressources`) — this is intentional/consistent, not a typo to "fix".

### Routing & auth composition

`src/main.rs` builds the App: `health`/`hello`/swagger UI are unauthenticated; everything else is nested under `web::scope("/api").wrap(JwtMiddleware)` (from `mairie360_api_lib::security`), configured via `endpoints::config` → `endpoints::v1::config`.

In `endpoints/v1/mod.rs`, the `/v1` scope is further wrapped with `AdminMiddleware` before `admin::config` is mounted — as currently written this `.wrap(AdminMiddleware)` applies to the whole `/v1` scope (auth, groups, roles, sessions, user, *and* admin), not just the `/admin` sub-scope; `admin/mod.rs`'s own `AdminMiddleware` wrap is commented out. Keep this in mind when adding routes under `/v1` — verify the intended auth level rather than assuming only `/admin/*` is admin-gated.

### OpenAPI docs

Docs compose bottom-up via `utoipa`'s `nest()`: each action's `doc.rs` → domain `doc.rs` → `v1/doc.rs` → `endpoints/swagger.rs`'s top-level `ApiDoc`, which also registers the `jwt` bearer security scheme and is served at `/swagger-ui/{_:.*}` and `/api-docs/openapi.json`.

### Shared library: `mairie360_api_lib`

External crate (crates.io) shared across mairie360 services; not vendored in this repo. Things pulled from it here:
- `pool::AppState` — holds the Postgres pool (`state.db_pool`) and Redis connection factory (`state.get_redis_conn()`).
- `security::{JwtMiddleware, AdminMiddleware}` — actix middlewares.
- `jwt_manager::generate_jwt`.
- `env_manager::get_critical_env_var` — reads required env vars and **panics** if unset (no silent fallback); all runtime config (`HOST`, `PORT`, `DB_*`, `REDIS_URL`, etc.) must be present in the environment/`docker-compose.yml`.
- `pool::redis::simple_key::secured::{handle_secure_get, handle_secure_post}` — encrypted key/value helpers on Redis (used e.g. for first-connection tokens).
- `database::db_interface::DatabaseQueryView` / `database::errors::DatabaseError` — the trait/error type the whole `database/` tree is built on.
- `test_setup::queries_setup::get_shared_db` — testcontainers-backed shared Postgres for integration tests.

### Email

`src/lib.rs` wraps `lettre` for SMTP sending (`build_email`/`send_email`). In dev, `SMTP_HOST=mailpit` skips TLS/auth and talks to the Mailpit container; production expects `SMTP_HOST`/`SMTP_USERNAME`/`SMTP_PASSWORD` and uses STARTTLS.

## CI/CD

`.github/workflows/cicd.yml` calls the shared reusable workflow `mairie360/CICD/.github/workflows/APIs_cicd.yml`, passing Postman collection/environment IDs for API testing. Dependency updates are managed by Renovate (`renovate.json`).
