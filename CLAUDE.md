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

Probes (MAIR-423): `GET /health` is the liveness probe (the process answers), `GET /ready` the readiness probe
(Postgres `SELECT 1` and a Redis round trip, 2 s each, `503 Not ready: <deps>` otherwise). Both live at the root
only, not under `/api`. At startup `main` waits for Postgres up to `DB_STARTUP_TIMEOUT_SECONDS` (default 60) and
exits with an error if it never answers, instead of serving `500`s. The compose stacks wait on `/ready`; the
chart's readiness probe (`Devops/Deploiment`) must point at it too.

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

The service under test in these three stacks is `image: ${IMAGE_REF}` (no `build:` block). CI sets `IMAGE_REF` to the
published `ghcr.io/mairie360/core-api:dev-<sha>` image; when it is empty the scripts build `core-api:local` from
`development.Dockerfile` first. That image is distroless (no shell, no curl), so readiness is a `core-ready` sidecar
polling `/health`, and dependent services wait for it with `service_completed_successfully`.

The three test stacks get a random `JWT_SECRET` per run from `stack_secrets.sh` (sourced by the `*_test.sh`
scripts, MAIR-428), which also signs `ADMIN_JWT` (`sub=1`, `role=admin`, 4 h): no secret or token is committed, and
the compose files refuse to start without them. The API refuses a missing, short (< 32 bytes) or well-known
`JWT_SECRET` at startup (mairie360_api_lib >= 2.0.0); only the dev `docker-compose.yml` keeps `b"secret"`, with
`JWT_ALLOW_WEAK_SECRET=true`. To replay the Postman collection by hand, pass `--env-var jwt_secret=<the stack's secret>`.

The ZAP scan is authenticated: `security-scan` injects `ADMIN_JWT` on every request, waits for the `seeder`
service (`init-test.sql`: plain `User` accounts `id=2` and `id=42`, user `id=1` is the Admin created by liquibase) and fails on
any alert not set to `IGNORE` / `OUTOFSCOPE` in `.zap/rules.tsv` (no `-I`). `-O http://core:3000` is required: the
spec's `servers` (localhost, development.mairie360.fr) are unreachable from the ZAP container. Keep `rules.tsv`
identical in every API. The scan fuzzes every field, so a `500` (value too long for its column, NUL byte, unmapped
constraint violation) fails the job: validate inputs, don't silence the alert. The XSS rules (40012, 40014, 40016,
40017) are ignored since MAIR-426: the API only serves JSON / plain text with `nosniff`, a name echoed with `<script>`
is data, and escaping it for HTML is the fronts' job (React escapes text; no `dangerouslySetInnerHTML`).

Both the ZAP and k6 stacks carry the OpenAPI coverage gate (MAIR-194) from mairie360/CICD `tests/`, available as
`cicd-repo/` (checked out by CI, cloned by the scripts at the pinned `cicd_version` otherwise, override with
`CICD_VERSION`; gitignored). ZAP runs with `--hook zap_hooks.py` and fails when an operation of the served spec was
never reached, or when an operation declaring `security(("jwt" = []))` only got 401/403 (the `/auth/**` routes
declare none: public). `load-test.js` is built on `coverage.js` and covers every operation (MAIR-195): GET handlers
run in the `reads` scenario (ramping up to 100 VUs) as the Admin against a group created in `setup()`, the other
methods in the `writes` scenario (10 VUs), each handler creating and deleting its own accounts, roles and groups so they are
order-independent (deleted accounts stay archived). The auth flows run end to end on throwaway accounts: register →
login `412` → `force_change_password` → login → refresh → revoke, and `forgot_password` → `reset_password` with the
token read from the Mailpit API (`MAILPIT_URL`). A third scenario, `login_rush`, replays the morning rush (up to 20
password logins/s on accounts created in `setup()`, `op:login_rush`, 1 s budget). One `p(95)` threshold per `op` tag
(200 ms reads, 500 ms writes), `http_req_failed == 0` (the expected `412` of the fixture logins is excluded through
`responseCallback`), `checks == 100%` (status and seeded rows) and no dropped iteration. Two load profiles (`K6_PROFILE`, passed by the compose file): `ci` (default) is what the 4 vCPU CI runner holds with the strict thresholds (30 readers, 4 writers, login rush at 8/s); `stress` is the high load (100 readers, 10 writers, 20 logins/s), run by hand with `K6_PROFILE=stress ./performance_test.sh`, not on every push. The k6 stack's seeder also runs `init-perf.sql`
(MAIR-474): 10 000 users, 51 000 sessions, 2 000 groups, so the lists are measured on a realistic volume (the ZAP
stack does not load it). The spec k6 reads is the one served by the image under test, saved into the
`openapi-spec` volume by `core-ready`. **Adding an endpoint = adding its handler in `load-test.js`** (k6 aborts at
init otherwise), nothing to do for ZAP. `init-test.sql` also seeds the rows of the spec's path examples (user 42,
group 3, role 6) so ZAP reaches real rows; the role examples point at 6 because the five base roles are protected
(`403` on delete), which the gate would read as an unauthenticated operation.

`tests/postman/collection.json` is a Postman v2.1 collection (importable in the app) and
`tests/postman/environment.json` its variables; the compose file overrides `baseUrl` with `--env-var` so the
committed default (`http://localhost:3000`) stays usable from a host shell. The scenario registers a fresh user
(unique e-mail generated in the collection pre-request script) so it is replayable against a persistent database.

`tests/routing_test.rs` checks that every `/api/v1` operation published by `ApiDoc` (the contract
`@mairie360/core-api-openapi` is generated from) hits a mounted actix route: Core does not normalise trailing
slashes, and utoipa replaces (does not merge) two `nest` entries that end on the same path, so operations sharing
a path must be merged into one document first (see `admin/users/doc.rs`).

`cargo cov` (what CI runs) fails under 60 % of lines covered, `src/endpoints/` included (MAIR-419): the
authorization and validation logic lives there, so a refusal path needs an integration test.
`tests/endpoints/access_denials.rs` walks the published contract and asserts that every `/admin` operation
refuses a non-admin (plain and percent-encoded path) and that every `jwt`-secured operation refuses a missing or
forged token, so a new operation is covered as soon as it is documented.

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

Request bodies and query strings are extracted with `endpoints::validation::{ValidatedJson, ValidatedQuery}`
instead of `web::Json` / `web::Query`: the view implements `Validate` (length matching the Postgres column, no
control character, e-mail / phone format; `<` and `>` are accepted since MAIR-426) and an invalid value answers
`400` naming the field before the handler runs. Document the rules in the view's `#[schema]` and the handler's
`400` response.

When adding a new endpoint, follow an existing sibling (e.g. `src/endpoints/v1/auth/login/` +
`src/database/auth/login/`) rather than inventing a new shape.

### Routing

`main.rs` mounts `/health` and `/ready` unauthenticated, Swagger UI + `/api-docs/openapi.json` only when
`SWAGGER_ENABLED=true` (MAIR-424: set in the dev, ZAP and k6 stacks, never in production), and `endpoints::config()`
(`v1::config()`) under `/api`. The template's `POST /` "Hello, world!" route is gone. In `main.rs`, the
whole `/api` scope is wrapped in `mairie360_api_lib::security::JwtMiddleware`; the `/admin` scope is
additionally wrapped in Core's own `endpoints::admin_guard` (MAIR-390), which checks the caller against the
database for every route mounted there and stores an `AdminUser` that every admin handler takes as argument.
Do not go back to the lib's `AdminMiddleware`: it decides from a regex on the raw path, which a
percent-encoded path (`/api/v1/%61dmin/...`) bypasses. Each domain module (`auth`, `groups`,
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

### Bounded lists (MAIR-425)

Lists that grow with the data take `endpoints::pagination::PageQuery` (`limit` 1-500, default 100, `offset`
0-1000000, `400` otherwise) through `ValidatedQuery`: `GET /sessions/history`, `GET /groups/`,
`GET /groups/{id}/users/`. Their query views have `new` (first page) and `page(id, PageQuery)`, with a stable
`ORDER BY`. The admin user list and the directory had their own bounds already (`page_size`, `limit`). Rate
limiting covers the public authentication routes (MAIR-390); the authenticated routes rely on the ingress.

### Ids (MAIR-422)

API ids are `u64`, Postgres ids `INT4`. Convert with `database::ids::{id_to_sql, id_to_sql_i64, id_from_sql}`,
never with `as`: an `as i32` wraps (`/user/4294967298/` read user 2), the helpers saturate so an out-of-range id
answers `404`. The `cast_possible_truncation` / `cast_possible_wrap` / `cast_sign_loss` lints are on.

### Database errors and logs (MAIR-421)

Never `map_err(|_| …)` a database error. `endpoints::db_error::log("<what the handler was doing>", &e)`
logs it through `tracing` and returns its `DbFailure`: `Conflict` (unique constraint → `409`), `NotFound` (no row
or foreign key → `404`), `Invalid` (SQLSTATE `22xxx` / `23xxx` → `400`), `Unavailable` (pool down → `503`),
`Internal` (anything else → `500`). Map the kinds the endpoint documents, answer the others with its generic `500`,
and never send the Postgres message to the client. Logs go through `tracing` (`tracing::error!` / `warn!`, no
`eprintln!` outside `src/bin/`): `telemetry::init()` always installs the stdout layer (filtered by `RUST_LOG`,
default `info`), with the OTLP export on top when it is enabled.

### Phone numbers (MAIR-480)

The phone is stored as `users.phone_country` (ISO 3166-1 alpha-2) + `users.phone_number` (national
significant number, digits only), Database v1.10.0. Requests send the number as typed (national
format with `phone_country`, or E.164) and `crate::phone::Phone::parse` validates it against the
numbering plan of the country (`phonenumber` crate); the stored country is the number's own (`+262`
sent with `FR` is `RE`). Partial updates go through `validation::check_phone_change`: absent keeps,
`null` or `""` clears, a value replaces; `phone_country` alone is a `400`. Responses give the phone
in E.164 (`phone::display`) next to `phone_country`. The database converts legacy writers (French
national number without a country, e.g. the `mairie360_api_lib` fixtures) itself.

### Multi-statement writes (MAIR-420)

A handler that checks then writes, or writes several rows, runs them in one transaction
(`state.get_smart_db().begin()`, committed explicitly; dropping it rolls back) or in one CTE
statement. The first read of a transaction locks what the checks depend on (`LockRoleQueryView`,
`IsUserActiveQueryView::locked`, `FOR UPDATE OF ac` in `GetAccessEntryQueryView`). Password changes
revoke the sessions in the same transaction through `session_revocation::revoke_all_user_sessions_in`
and publish them to Redis only after the commit. `tests/endpoints/transactions.rs` checks the rollbacks.

### Sessions / auth flow

Login (`endpoints/v1/auth/login/endpoint.rs`) always checks the password first (archived and
passwordless accounts answer the same `401`, after the same argon2 work). On a user's first connection a
**correct** password returns `412 Precondition Failed` with a one-time token (stored in Redis) instead of
logging in, forcing a password change via `force_change_password`. On success it issues a JWT
(`session_jwt::generate_session_jwt`) plus an opaque refresh token: only its SHA-256 digest is stored in
`sessions.token_hash` (`refresh_token::hash`), and `POST /sessions/refresh` rotates it (the JSON body returns
the new token, the old one stops working). Hash any refresh token before handing it to a session query view.

Argon2 (MAIR-474): hash and verify passwords through `crate::passwords::{hash, verify}`, which run on actix's
blocking pool (`web::block`), never by calling `mairie360_api_lib::password::{hash_password, verify_password}` from a
handler: inline, each hash blocks the worker thread and every request queued on it.

Security knobs added by MAIR-390, all optional:

- `TRUSTED_PROXIES` (`client_ip.rs`): comma-separated addresses / CIDR ranges whose `X-Forwarded-For` /
  `Forwarded` is trusted; loopback + private ranges by default, empty string = trust none. Read client
  addresses with `client_ip::client_ip`, never `realip_remote_addr()`.
- `RATE_LIMIT_ENABLED=false` (`rate_limit.rs`) turns off the in-memory per-replica budgets of login,
  keycloak, forgot/reset/force_change_password and refresh (`429` + `Retry-After`). Handlers take
  `Option<web::Data<RateLimits>>`; tests that do not register it are not limited.
- `SMTP_INSECURE=true` allows a plaintext SMTP relay other than `mailpit` / `localhost`; otherwise STARTTLS.

### Database schema

Postgres schema is **not** managed in this repo — it lives in a separate Liquibase migrations image
(`ghcr.io/mairie360/liquibase-migrations`, run as the `liquibase` service in `docker-compose.yml`) applied
against the `ghcr.io/mairie360/database` image; its source is `../../Devops/Database`.

### Docs referenced in this repo

- `API.md` — informal endpoint list (partial/stale in places, e.g. missing rows).

### Observability (MAIR-131)

`src/telemetry.rs` exports traces over OTLP/HTTP (protobuf), opt-in and driven by the standard
`OTEL_*` variables: nothing changes unless `OTEL_EXPORTER_OTLP_ENDPOINT` (or
`OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`) is set, and `OTEL_SDK_DISABLED=true` forces it off. The
endpoint is an agent that relays to Scaleway Cockpit (OTel Collector or Grafana Alloy, e.g.
`http://alloy:4318`, `/v1/traces` is appended); `OTEL_SERVICE_NAME` defaults to `core-api`,
`OTEL_EXPORTER_OTLP_HEADERS` carries a token when the endpoint needs one. `RUST_LOG` (default
`info`) filters the stdout logs, always on since MAIR-421. A failure to build the exporter is
printed and never stops the API.

- `main.rs` wraps the app in `tracing_actix_web::TracingLogger` (outermost, so requests refused by
  the JWT / session guards get a span too; it replaces `middleware::Logger`). Root spans are named
  `<METHOD> <route pattern>`, carry `http.*` attributes and continue an incoming `traceparent`.
  `http.target` includes the query string (directory search terms): do not put secrets in query
  strings.
- The SQL of `mairie360_api_lib` (sqlx) appears as span **events** (`db.statement` with `$n`
  placeholders, never the bound values, plus `elapsed` and the row counts), not as child spans:
  sqlx reports statements as `tracing` events of the `sqlx::query` target. Real `db` spans need a
  change in `API_lib`.
- The `opentelemetry*`, `opentelemetry-otlp`, `opentelemetry_sdk`, `tracing-opentelemetry` and
  `tracing-actix-web` versions are coupled (`tracing-actix-web` 0.7 supports OpenTelemetry up to
  0.32, `tracing-opentelemetry` 0.33): bump them together, never one alone.
- CPU/RAM of the pod are not app metrics: they come from the cluster agent, not from this crate.
- `tests/endpoints/telemetry.rs` asserts the span, the continued trace id and the SQL events
  against an in-memory exporter (Docker needed like the other integration tests).

## CI

`.github/workflows/cicd.yml` delegates to the reusable `mairie360/CICD` workflow (`APIs_cicd.yml`) on every
push, with no input other than the package name and the CICD version. The `uses:` is pinned by commit SHA with the
tag in a comment, `cicd_version` carries the same tag, and Renovate bumps both in one `mairie360/CICD` group; only
the two secrets the workflow declares are passed (no `secrets: inherit`). Both Dockerfiles build on the template's
Rust toolchain pinned by digest, with `--locked` and a dependency-cache layer (MAIR-427); the `integration_tests`, `integration_and_security` and `performance_isolated` jobs run the three `*_test.sh` scripts with `IMAGE_REF` set to the `dev-<sha>` image published by `release-dev` (newman, no Postman account involved). `auto-approve.yml` auto-approves Renovate
PRs.

## Pull request reviewers

Every PR requests a review from the whole team, minus its author: `CarolinHugo`, `LAURETbenjamin`, `MathTek` and `Quentintnrl` (`gh pr create … --reviewer CarolinHugo,LAURETbenjamin,MathTek`). `.github/CODEOWNERS` makes GitHub request them automatically as well.
