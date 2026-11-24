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
docker compose up --build --watch   # full stack: postgres, liquibase migrations, redis, core (hot reload), fake-resend, nginx
```

Probes (MAIR-423): `GET /health` is the liveness probe (the process answers), `GET /ready` the readiness probe
(Postgres `SELECT 1` and a Redis round trip, 2 s each, `503 Not ready: <deps>` otherwise). Both live at the root
only, not under `/api`. At startup `main` waits for Postgres up to `DB_STARTUP_TIMEOUT_SECONDS` (default 60) and
exits with an error if it never answers, instead of serving `500`s. The compose stacks wait on `/ready`; the
chart's readiness probe (`Devops/Deploiment`) must point at it too.

The dev stack is reached via nginx at `http://development.mairie360.fr`. `core` requires `HOST`, `PORT`,
`REDIS_URL`, `DB_USER`, `DB_PASSWORD`, `DB_HOST`, `DB_PORT`, `DB_NAME` and `JWT_TIMEOUT`
— all fetched with `get_critical_env_var` (panics on missing var) — plus the session keys and the e-mail
variables below, see `docker-compose.yml` for the dev values. The Postgres URL is assembled by
`database::pg_url::build_pg_url`, which percent-encodes user, password and database name, so
`DB_PASSWORD` may contain any character.

Session keys (MAIR-680): Core is the only service that signs session JWTs. `JWT_PRIVATE_KEY` (Ed25519 PKCS#8
PEM, `openssl genpkey -algorithm ed25519`) + `JWT_PUBLIC_KEY` (its public key, `openssl pkey -pubout`; a second
`PUBLIC KEY` block during a rotation) make Core sign `EdDSA` JWTs with `kid`, `iss` `mairie360-core` and `aud`
`mairie360`; the other APIs and BFFs only get `JWT_PUBLIC_KEY`. Since `mairie360_api_lib` 6.0.0 both are required
on Core: there is no `JWT_SECRET` any more (ignored when set) and every HS256 token is refused. Signing and
verification are the lib's (`session_jwt.rs` delegates to `mairie360_api_lib::jwt_manager`).
`GET /api/v1/auth/jwks` publishes the public keys (informational: nothing fetches it at runtime). The test stacks
get a fresh key pair per run from `stack_secrets.sh`; the dev compose takes them from the shell (`source
./stack_secrets.sh` before `docker compose up --watch`, it refuses to start otherwise). The `cargo test` tests sign
with the lib's test key pair (`init_test_jwt_env()`, `tests/common/session_tokens.rs` for chosen claims).

WebAuthn (MAIR-810): `WEBAUTHN_RP_ID` + `WEBAUTHN_RP_ORIGIN` enable passkeys (see "Passkeys" below); the dev and test composes use
`localhost` / `http://localhost`.

Second factor key (MAIR-805): `TOTP_ENCRYPTION_KEY` (base64 of 32 bytes, `openssl rand -base64 32`) seals the TOTP
secrets with AES-256-GCM before they reach `user_totp`. Without it `main.rs` registers no `SecretCipher`: the
`/user/me/2fa/*` setup routes answer `503` and an account that already has a factor cannot log in (`503`, never a
session without the factor). Losing the key loses every enrolled secret (administrators reset them). The test
stacks get a random key per run from `stack_secrets.sh`; the dev compose has a throwaway one.

E-mails (MAIR-522, `src/emails/`): every e-mail is a **published Resend template** filled in by Core,
never a body written in the code. `emails::PasswordResetEmail` implements the lib's `EmailTemplate` (alias
`account-password-reset`, variables `USER_FIRST_NAME`, `RESET_URL`, `EXPIRE_MINUTES`, optional
`MAIRIE_NAME`); `emails::AccountEmails` (app data built by `main`, which refuses to start without it) sends
it through `mairie360_api_lib::email::ResendMailer`. The sender is always `no-reply@<EMAIL_DOMAIN>`, no
`Reply-To`: only the domain is configurable. Variables: `RESEND_API_KEY`, `EMAIL_DOMAIN` (verified in
Resend), `PUBLIC_URL` (login front, the reset link is `<PUBLIC_URL>/reset-password?token=…`), optional
`EMAIL_SENDER_NAME` and `MAIRIE_NAME`. The reset token lives 15 minutes (`FORGOT_PASSWORD_TTL_SECONDS`)
and is dropped when Resend refuses the e-mail. A new e-mail = a template published in Resend + a struct
implementing `EmailTemplate` here. The compose stacks run a fake Resend (WireMock, `tests/fake-resend/`)
reached through `RESEND_BASE_URL`; the dev stack exposes its journal on `localhost:8025/__admin/requests`.
The integration tests use the one of `tests/common/fake_resend.rs`.

Account invitation (`src/emails/account_invitation.rs`): `POST /api/v1/admin/users/` e-mails the Resend template
`account-invitation` (`USER_FIRST_NAME`, `SET_PASSWORD_URL`, `EXPIRE_DAYS`, optional `MAIRIE_NAME`) after the insert. The
link is `<PUBLIC_URL>/reset-password?token=…`, valid `INVITATION_TTL_SECONDS` (7 days), and its token is stored like a
forgot-password token **without** the per-address key (`<email>/forgot_password_token`), so a pending invitation never
silences a later `POST /auth/forgot_password`; `POST /auth/reset_password` consumes it. A failed e-mail is logged and never
undoes the account. The template must be published in Resend before deploying. There is no Keycloak any more: no sign-in,
no migration binary, no mirroring of the administration routes (the account system is Core alone).

E-mail change (`src/email_change.rs`): `PATCH /api/v1/user/me/` with a new `email` (and `current_password`, rate limited per user as
before) no longer writes it. The address is recorded as a pending request in Redis for `EMAIL_CHANGE_TTL_SECONDS` (1 hour:
`<token>/email_change_request` = `<user id>|<new address>`, `<user id>/email_change_token` = the token, `<token>/email_change_spent` = `SET NX` marker of a used token, so a user has one pending
request, a newer one replaces it once its e-mail was delivered (a failed delivery keeps the request already sent), a token that is not the
current one of its user is refused, and sending the current address again cancels it) and the Resend template
`account-email-change-confirm` (`USER_FIRST_NAME`, `CONFIRM_URL`, `NEW_EMAIL`, `EXPIRE_MINUTES`, optional `MAIRIE_NAME`) is
sent to the NEW address; the account keeps its address until `POST /api/v1/auth/confirm_email {token}` (public, single-use,
`401` for an unknown / used / expired token or an archived account, `409` when another account took the address meanwhile, rate
limited, its own `confirm_email_per_ip` budget) writes it and sends the alert `account-email-changed` (`USER_FIRST_NAME`, `NEW_EMAIL`,
`DATE`) to the OLD address; a failed alert is logged, never undoes the change. The link is `<PUBLIC_URL>/confirm-email?token=…`:
that page of the login front must exist before the feature is used for real. The address is checked against every account,
archived ones included and whatever the letter case (`EmailInUseQueryView`), and the `409` never says whose it is. The mail is
sent **before** anything is written: a refused or unsent confirmation (`500`) changes nothing, not even the other fields of the
same request, which are otherwise written at once. `GET /api/v1/user/me/` answers the waiting address as `pending_email`. Both
Resend templates must be published before deploying. Tests: `tests/endpoints/mair_696_email_change.rs` and the newman folder
"E-mail change confirmation".

Accounts without a password (`src/invitations.rs`, `src/bin/invite_passwordless.rs`): the accounts the former Keycloak
synchronisation created have `users.password IS NULL`. The `invite_passwordless` binary (shipped as
`/app/invite-passwordless` in the image) lists the non-archived ones and e-mails each the same invitation
(`emails::invite_account`, shared with `POST /admin/users/`). Without argument it is a dry run (counts, sends nothing);
`--send` sends, prints a JSON report (ids only, never an address) and exits `1` when an e-mail failed: run it again, it
invites whoever still has no password with a fresh 7-day link. Same variables as the API (`DB_*`, `REDIS_URL`,
`RESEND_API_KEY`, `EMAIL_DOMAIN`, `PUBLIC_URL`). Run it before each instance switches
(`kubectl exec deploy/<core> -- /app/invite-passwordless`, then `--send`). Tests: `tests/endpoints/passwordless_invitation.rs`.

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

The three test stacks get a random Ed25519 key pair per run from `stack_secrets.sh` (sourced by the `*_test.sh`
scripts, MAIR-428, MAIR-680), which also signs `ADMIN_JWT` (`sub=1`, `role=admin`, 4 h, `EdDSA` like Core): no key or
token is committed, and the compose files refuse to start without them. The newman sandbox cannot sign Ed25519, so the
integration stack runs a test-only signer (`jwt-signer`, `tests/postman/jwt-signer.js`, same image as newman, internal
network only) that signs the JWTs the collection forges with the run's key; the collection reaches it through
`jwtSignerUrl`. To replay the collection by hand, run that script with the stack's `JWT_PRIVATE_KEY`
(`node tests/postman/jwt-signer.js`, port 8080) and pass `--env-var jwtSignerUrl=http://localhost:8080`.

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
token read from the fake Resend's request journal (`FAKE_RESEND_URL`). A third scenario, `login_rush`, replays the morning rush (up to 20
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

Split of the tests (MAIR-262): `cargo test` (unit and handler tests) validates the code, newman validates that the
running image works. The collection holds, for **every operation of the spec**, its success case and one case per
documented `400` / `401` / `403` / `404` / `409` / `412`, chained in business journeys (account creation → first
login → password change → sessions → profile → groups, members and ACLs → roles → archive), each case checking the
status and what the front relies on. The `429` / `500` / `502` / `503` cannot be produced on purpose on a healthy
stack and stay in the handler tests. Three collection-level scripts, identical in every API collection, back it:

- the collection pre-request loads `/api-docs/openapi.json` once (the stack sets `SWAGGER_ENABLED=true`), and the
  collection test checks every answer against it: documented operation, documented status, content type, and the
  JSON body against the response schema (`$ref`s inlined);
- `Every secured operation refuses a missing JWT` replays every `jwt`-secured operation of the spec without a token
  and expects a documented `401 text/plain`;
- `OpenAPI coverage`, the last request, fails when an operation has no success case or a documented status above
  was never answered, and when an entry of the collection variable `openapi_coverage_exceptions` (`"METHOD /path
  STATUS": "reason"`, `2xx` for the success) is no longer needed. The exceptions are the directory `403`s, only produced with
  `DIRECTORY_GUEST_ACCESS=none` (`tests/endpoints/access_matrix.rs`). The run ends by dropping `openapi_spec` and
  `openapi_covered`, so a next run in the Postman app starts from the spec of the image it tests.

A new endpoint therefore fails the integration stack until the collection tests it. The refusal cases need a second
plain account: the collection creates it through the admin route and forges its JWT (`jwt_other`) like the Admin's,
through the stack's signer.
The password reset reads its token from the fake Resend's journal (`fakeResendUrl`). The public auth routes are
rate limited per address (`429`): replay the collection on a fresh stack (`docker compose down -v`), as CI does.

`tests/routing_test.rs` checks that every `/api/v1` operation published by `ApiDoc` (the contract
`@mairie360/core-api-openapi` is generated from) hits a mounted actix route: Core does not normalise trailing
slashes, and utoipa replaces (does not merge) two `nest` entries that end on the same path, so operations sharing
a path must be merged into one document first (see `admin/users/doc.rs`).

`tests/operation_ids_test.rs` (MAIR-692) fails when two operations share a client name: orval names the generated
functions after the `operationId` (the handler name by default, camel-cased), and a clash renames an existing one
(`logout` → `logout2`) without any compile error in the consumers. Give a new handler whose name is taken an
`operation_id`.

`cargo cov` (what CI runs) fails under 60 % of lines covered, `src/endpoints/` included (MAIR-419): the
authorization and validation logic lives there, so a refusal path needs an integration test.
`tests/endpoints/access_denials.rs` walks the published contract and asserts that every `/admin` operation
refuses a non-admin (plain and percent-encoded path) and that every `jwt`-secured operation refuses a missing or
forged token, so a new operation is covered as soon as it is documented.

Integration tests need **Docker running** — `get_shared_db()` (from `mairie360_api_lib::test_setup`) spins up a
shared Postgres testcontainer on first use and hands back its **superuser** URL, used by the fixtures only
(`tests/common::get_raw_pool`). Everything that runs Core's code connects as the `core_api` role, like in
staging (MAIR-556): `tests/common::get_pool()` (query views) and `AppState::new(…, core_db_url().await)`
(endpoints). A query outside the grants of Database's `api_grants.sql` then fails the test with
`permission denied`: add the grant there, never connect the code under test as the superuser. The compose stacks
connect `core` as `core_api` too (`-Dcore_api_password` passed to Liquibase); postgres, liquibase and the seeder
keep the superuser.

Tests run in parallel (MAIR-1002): there is no `#[serial]` for the database. Every test gets a database of its
own: the first helper it calls (`tests/common::own_db()`, behind `core_db_url()`, `get_pool()` and the
`own_db_url()` the fixtures use with `get_raw_pool`) creates a copy of the seeded template with
`mairie360_api_lib::test_setup::isolated::isolated_db()` (about 0.2 s) for the thread of the test, and the end of the
test drops it. A test therefore reads and writes freely (counts, sequence resets, migrations over every account)
without reaching another one; the seeded ids (`ADMIN_ID`, `ALICE_ID`...) are the same in every copy. Never connect to
`get_shared_db()` from a test: the shared database is only the template's source. `isolated_redis()` gives a Redis
client with its own key prefix on one container shared by the process. A test that sets a process environment variable
or shares the fake Resend server keeps a named `#[serial(<group>)]` (`process_env`, `fake_resend`). The dependencies are
compiled with `opt-level = 3` (`[profile.dev.package."*"]`): the code of the crate stays in debug, which is what the
coverage measures, while argon2 and the TLS stack no longer dominate the run.

OpenAPI client generation (consumed by other repos, not by this API itself):

```bash
npm install
npx orval   # reads openapi.json, writes generated/ (per orval.config.js)
```

## Architecture

### Two parallel trees: `database/` and `endpoints/v1/`

Business logic is split into two mirrored trees under `src/`, one per resource/action:

- `src/database/<domain>/<action>/` — the DB access layer, one folder per query:
  - `mod.rs` — re-exports the view types (`pub use view::…`)
  - `view.rs` — a `*QueryView` implementing `mairie360_api_lib::database::db_interface::ApiRequestDto`:
    `query_sql()` returns the SQL with `$n` placeholders, `query_params()` the bound `QueryParam`s
    (ids go through `crate::database::ids::id_to_sql` for INT4 columns, which saturates at
    `i32::MAX`, and `id_to_sql_i64` for INT8 ones, e.g. `RemoveAccessQueryView`). Two separate cache
    mechanisms: `cache_key` / `cache_ttl` cache this view's own result in `SmartDatabase`, while
    `invalidated_read_caches` bumps the namespaces of the lib's `ReadCache` (`cached(&key, …)`,
    MAIR-524) after a write; one never invalidates the other. A `*QueryResultView` (`serde::Deserialize`)
    for the row, usually read as one JSON object (`SELECT row_to_json(t) FROM (…) t`).
  There is no per-query `query.rs` any more: the views are executed by the lib's `SmartDatabase`.

- `src/endpoints/v1/<domain>/<action>/` — the HTTP layer, same triad idea:
  - `endpoint.rs` — the actix handler (`#[get]`/`#[post]`/etc + `#[utoipa::path]`), an `*Error` enum
    implementing `ResponseError` for domain-specific failure→status mapping, and the handler logic that calls
    a `database::` view through `state.get_smart_db()` (`fetch_one` / `fetch_all` / `fetch_scalar` /
    `execute`, `begin()` for a transaction) where `state: web::Data<AppState>`
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

`AppState` (`mairie360_api_lib::state`) is built once in `main.rs` from `REDIS_URL` and the Postgres URL
(`database::pg_url`) and shared via `web::Data`. `state.get_smart_db()` is the only way to the database: a
`SmartDatabase` around the lib's `sqlx` pool (sized by the `DB_POOL_*` variables, see "Postgres pool
settings") that runs `ApiRequestDto` views, maps errors to `DbError` (`ApiLibError::Database`) and keeps the
directory read cache. `state.get_redis()` gives the Redis client (session revocation list, readiness probe,
the realm console's single-use links).

`sqlx` is a direct dependency only to read the SQLSTATE of a `DbError::Sqlx` (same version as the lib, no
default features); the tests enable its Postgres driver. `tokio-postgres` is still listed in `Cargo.toml`
but no source file uses it.

### Bounded lists (MAIR-425)

Lists that grow with the data take `endpoints::pagination::PageQuery` (`limit` 1-500, default 100, `offset`
0-1000000, `400` otherwise) through `ValidatedQuery`: `GET /sessions/history`, `GET /groups/`,
`GET /groups/{id}/users/`. Their query views have `new` (first page) and `page(id, PageQuery)`, with a stable
`ORDER BY`. The admin user list and the directory had their own bounds already (`page_size`, `limit`). Rate
limiting covers the public authentication routes (MAIR-390) and, since MAIR-746, the check of the current password
by `PATCH /user/me/` (e-mail change, `password_check_per_user`: 5 checks per user per 15 minutes, right or wrong);
the other authenticated routes rely on the ingress.

### Ids (MAIR-422)

API ids are `u64`, Postgres ids `INT4`. Convert with `database::ids::{id_to_sql, id_to_sql_i64, id_from_sql}`,
never with `as`: an `as i32` wraps (`/user/4294967298/` read user 2), the helpers saturate so an out-of-range id
answers `404`. The `cast_possible_truncation` / `cast_possible_wrap` / `cast_sign_loss` lints are on.

A path id that does not parse (`/admin/users/abc/`) answers `400 text/plain` `Invalid path: ids are integers.`
(`endpoints::config` registers a `PathConfig`, MAIR-262; the routes behind `access_guard_middleware` answer the
lib's `Invalid ID format in URL`), and every operation with a path id documents that `400`:
`tests/endpoints/path_ids.rs` walks the contract. Plain-text answers without a content type get
`text/plain; charset=utf-8` from `endpoints::headers::default_headers()` (with `nosniff`), wired in `main.rs`.

### Database errors and logs (MAIR-421)

Never `map_err(|_| …)` a database error. `endpoints::db_error::log("<what the handler was doing>", &e)`
logs it through `tracing` and returns its `DbFailure`: `Conflict` (unique constraint → `409`), `NotFound` (no row
or foreign key → `404`), `Invalid` (SQLSTATE `22xxx` / `23xxx` → `400`), `Unavailable` (pool down → `503`),
`Internal` (anything else → `500`). Map the kinds the endpoint documents, answer the others with its generic `500`,
and never send the Postgres message to the client. Logs go through `tracing` (`tracing::error!` / `warn!`, no
`eprintln!` outside `src/bin/`): the lib's `init_telemetry("core-api")` always installs the stdout layer
(filtered by `RUST_LOG`, default `info`), with the OTLP export on top when it is enabled.

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

### Directory read cache (MAIR-524)

`GET /api/v1/user/` (the directory, `list_directory_users`) is served through the lib's
`ReadCache`: `database::users::list_directory::DIRECTORY_CACHE` (namespace `directory`, 60 s,
`DIRECTORY_CACHE_TTL_SECONDS`) with `ReadCache::shared` keys, because the page does not depend on
the caller (no ACL, same rows whoever asks). `ListDirectoryUsersQueryView::cache_key` only applies
normalisations the SQL ignores (search trimmed, `ids` / `group_ids` sorted and deduplicated); the
search case is kept. Entries are `core-api:read-cache:directory:<generation>:shared:<sha256>` under
the chart ACL. **Every write view that changes what the directory shows returns
`INVALIDATES_DIRECTORY` from `invalidated_read_caches`**: `RegisterUserQueryView` (admin creation),
`PatchUserQueryView` (admin patch and `PATCH /user/me/`), `DeleteUserQueryView` (archive),
`AddRolesQueryView` / `RemoveRolesQueryView`, `PatchRoleQueryView` / `ChangeRoleQueryView` /
`DeleteRoleQueryView` (role names), `CreateGroupQueryView` (the trigger adds the owner),
`DeleteGroupQueryView` (cascade), `AddUserToGroupQueryView` / `DeleteUserFromGroupQueryView`. A new
write on `users`, `user_roles`, `roles.name` or `group_members` must do the same; inside a
transaction the namespace is bumped on `commit()`. Changes outside Core (raw SQL, another service)
show up after the TTL. `tests/endpoints/directory_cache.rs` covers the hits and every invalidation
against the ACL Redis. Without a reachable Redis (most tests) the cache is bypassed.

### One role per user (MAIR-564)

The database keys `user_roles` by the user (Database 3.1.0, `mair-564-01`): a user holds at most
one role. `POST /api/v1/admin/users/{userId}/roles/` **replaces** the role
(`AddRolesQueryView`: `INSERT … ON CONFLICT (user_id) DO UPDATE … RETURNING`, no row = the role was
already held, `409`). An account registered without a role holds `Guest`; the schema's
`tr_user_roles_replace_default` lets a raw `INSERT INTO user_roles` replace that default Guest,
any other second role is a `23505`. The responses expose the role as `role` (`/user/{id}/`,
`/user/me/`, the directory, the admin list and record); the `roles` lists are deprecated (at most
one entry) and kept for the BFFs until they move to `role`. The upsert needs the new key: Core
must run on a Database image that has `mair-564-01`.

### Last administrator (MAIR-678)

An instance keeps at least one active administrator (role named `Admin`, account not archived, as
`is_admin()` reads it). The role grant, the role revocation and the archiving
(`admin/users/{id}/roles` POST / DELETE, `DELETE admin/users/{id}/`) run their write in a
transaction that first locks the `Admin` role row (`endpoints::last_admin::lock`, `FOR NO KEY
UPDATE`), then checks `any_left` after the write and rolls back with `409`
`LAST_ADMIN_MESSAGE` when none is left. A new write that can demote or archive a user must do the same.
`tests/mair_678_last_admin_guard.rs` is its own test binary (it controls how many administrators
exist). The erasure is guarded by `anonymize_user()` itself (`23001`). A system role (`can_be_deleted =
false`, `Admin` first) cannot be unprotected: `PATCH` / `PUT admin/roles/{id}` answer `403` to
`can_be_deleted: true`, so the role can be neither renamed nor deleted afterwards. A custom role
that an administrator protects becomes a system role for good.

### Sessions of a user and suspension (MAIR-584, MAIR-904)

The administration reads the sessions of a user in `GET /api/v1/admin/users/{userId}/` (`sessions`, latest 100) and
closes them with `DELETE /api/v1/admin/users/{userId}/sessions` (every active one; republishes every session revoked
within `JWT_TIMEOUT`, `503` when Redis refuses, so repeating it is effective) and `DELETE
/api/v1/admin/users/{userId}/sessions/{sessionId}` (one, `404` when unknown, another user's or already ended).
`POST /api/v1/admin/users/{userId}/suspend` sets `users.status = 'suspended'` (Database `mair-904-01`) and revokes
every session in one transaction guarded like the last-administrator writes (an administrator cannot suspend their
own account: `409`); `POST .../reactivate` gives back `offline`. A suspended account keeps its roles, groups and
factors, but the sign-in views (`LoginUserQueryView`, `GetActiveUserIdByEmailQueryView`, `ResetRecipientQueryView`,
the passkey lookups) skip it like an archived one, `HasActiveAdminQueryView` does not count it, and Database's
`is_admin()` / `check_access()` deny it. API_lib is unchanged: the revoked sessions reach every API through the
revocation list. The admin list rows carry `account_status` (`active` / `invited` / `inactive` = archived / `suspended`, computed by
`admin_account_status_sql!`), `active_sessions`, `last_login_at` and `two_factor { methods, last_verified_at }` (never a
secret); `GET /admin/users/` filters on `account_status` and `role_id` and sorts with `sort=name|-name`
(`AdminUsersFilter`, four static SQL variants: searched or not × ascending or descending). Tests: `tests/endpoints/mair_584_admin_user_sessions.rs` (needs a Database image
with `mair-904-01`) and the newman folder "Sessions and suspension of a user".

### Modules of a role, services, group colours (MAIR-916, MAIR-923)

Database `mair-916-01` / `mair-923-01` / `mair-923-02` (v4.0.0). A role lists the modules it opens
(`role_modules`, ids of `role_modules::MODULES`: `dashboard`, `projects`, `calendar`, `messages`,
`training`, `admin`, `settings`): the admin role routes take an optional `modules` list (a new role
without one opens `dashboard`), `GET /admin/roles/` and the roles of `/user/me/` return them in
navigation order, and `GET /user/me/` answers the caller's `modules` (the shared navigation hides the
others; the business APIs do not check them). Taking `admin` away from the `Admin` role is refused by
the schema trigger (`23514` → `403`). Services of the mairie: `GET` / `POST /admin/services/`,
`PATCH` / `DELETE /admin/services/{serviceId}` (name unique whatever its case, `409`; deleting one
leaves its agents without service) and `PUT /admin/users/{userId}/service {service_id | null}` (`404`
for an unknown user or service); the admin list and record carry `service {id, name}` and the admin
search also matches the name of the service. Groups take an optional `color` of the charter palette
(`groups::color::GROUP_COLORS`, `400` otherwise; `null` in a `PATCH` gives back the default) and
answer `project_count` (rows of `project_groups`, read through the `SELECT (group_id)` grant). Tests:
`tests/endpoints/mair_916_923_admin_v2.rs`, newman folder "Services, role modules and group colours".

### Second factor (MAIR-805)

TOTP (RFC 6238, SHA-1, 6 digits, 30 s, ±1 step) plus 10 single-use backup codes, all in `second_factor.rs`
(primitives, no I/O), `two_factor.rs` (service: verification, anti-replay by `user_totp.last_used_step` spent
with an atomic `UPDATE … RETURNING`, backup codes stored as Argon2 hashes) and `database/second_factor.rs`
(query views). Routes: `/user/me/2fa/` (status, `setup`, `confirm`, `disable`, `backup-codes`), `POST
/auth/login/2fa` and the administrator reset `DELETE /admin/users/{userId}/2fa` (revokes the sessions, `AUDIT`
log line). A password login on a protected account answers `202 {"status":"2fa_required","token"}` with **no
session**; the token lives in Redis (`2fa_login/<token>`, 5 minutes, burnt after 5 wrong codes) and opens no
route. `reset_password` goes through the same second step: owning the mailbox is not a second factor.
Redis `secure_set_ex` only writes an absent key (`SET NX`); use `set_ex` to overwrite.

Races of the second factor (review of MAIR-805), each pinned by a `mair_805_*` test of `tests/endpoints/second_factor.rs`:
the login token carries its absolute `expires_at` (a wrong code rewrites the counter with what is left of the TTL, it
never renews it: the lib's `Redis` has no `KEEPTTL`), it is spent by one `SET NX` on `2fa_spent/<token>` (no
`GETDEL` in the lib), the anti-replay step and the confirmation are written `WHERE secret_encrypted = <the secret that
was checked>`, `issue_backup_codes` locks the account row first, and the second step writes the session in a transaction
that holds `FOR SHARE` on the factor (`LockActiveTotpQueryView`): the administrator reset (`DELETE FROM user_totp`) waits
for it, then revokes that session too. The revocation list publication has no outbox: the table is the retry state,
`DELETE /admin/users/{id}/2fa` republishes every session revoked within `JWT_TIMEOUT`
(`RecentlyRevokedSessionsQueryView`) and answers `503` when Redis refuses, so repeating the call is effective.
`reset_password` checks that the instance can verify the account's factor **before** it changes anything.

### Mandatory second factor (MAIR-808)

The instance policy is the single row of `two_factor_policy` (Database 3.6.0: `required`, `totp_allowed`,
`passkey_allowed`, `backup_codes_allowed`), read and written by Core alone (`database/two_factor_policy.rs`,
`two_factor::load_policy`), changed by administrators through `GET` / `PUT /api/v1/admin/two-factor-policy` (every
change is audited by a trigger in `two_factor_policy_audit_log`, plus an `AUDIT:` log line). `passkey_allowed: true`
is refused (`400`) while the instance has no WebAuthn configuration (`Passkeys` app data, MAIR-810); at least one mode must stay allowed.
A factor whose mode was removed does not count (`verify_code_under_policy`: TOTP refused when `!totp_allowed`, a backup
code when `!backup_codes_allowed`). With `required` and no usable factor, `login` and `reset_password` open **no
session** and answer `202 {"status":"2fa_setup_required","token","policy":{totp_allowed,passkey_allowed,backup_codes_allowed}}` (the policy lets the
Login screen offer only what is allowed; `2fa_required` carries none); the token lives in Redis (`2fa_setup/<token>`,
5 minutes, burnt after 5 wrong codes) and only opens the public `POST /auth/2fa/setup {token}` (secret + `otpauth_uri`,
the same enrolment as `/user/me/2fa/setup`) and `POST /auth/2fa/confirm {token, code}` (activates the factor, opens the
session like a login and returns the ten backup codes once). `POST /user/me/2fa/disable` answers `403` while the policy is
`required`. The policy table has no foreign key on purpose: the test tooling truncates `users` with CASCADE.
`load-test.js` runs the four operations in an isolated one-iteration `policy` scenario (a mandatory policy would make every
other login answer 202) and restores the optional policy.

### Passkeys (MAIR-810)

WebAuthn through `webauthn-rs` (`src/passkeys.rs`, relying party built by `Passkeys::from_env`): `WEBAUTHN_RP_ID` (the
site's registrable domain, e.g. `mairie360-eip.fr`) and `WEBAUTHN_RP_ORIGIN` (the origins the browser signs for, comma-separated: passkeys are registered from the settings front
and used from the login front, e.g. `https://login.mairie360-eip.fr,https://settings.mairie360-eip.fr`) must be set **together**; with neither, `main.rs` registers no `Passkeys`, the passkey
routes answer `503` and `PUT /admin/two-factor-policy` refuses `passkey_allowed: true`. Passkeys are discoverable
(resident) credentials (the options sent to the browser carry `residentKey: "required"`, added in `require_resident_key` because `webauthn-rs` only exposes it for attested registrations; the software authenticator of the tests cannot hold one, so `for_software_authenticator` lowers it), user verification required, no attestation (synchronised passkeys are accepted). The key of an
agent is stored as the `webauthn-rs` `Passkey` JSON in `user_passkeys` (Database 3.7.0, one row per passkey, `credential_id`
unique, `ON DELETE CASCADE`); the user handle given to the authenticator is a v5 UUID of the user id (`user_handle`).
Ceremonies live in Redis, two minutes, read once (`passkeys::begin_*` / `take_*`: the first attempt burns them, right or
wrong). Routes: public `POST /auth/passkey/options` (challenge, no account named: nobody can be enumerated) and
`POST /auth/passkey/setup/options` + `POST /auth/passkey/setup` (mandatory enrolment by passkey: the `2fa_setup_required` token is the credential, the ceremony is kept per account, a refused answer counts like a wrong code and the token is burnt at the fifth, success spends the token, opens the session like `login`, sends the security e-mail and issues no backup codes), `POST /auth/passkey` (identifies the account by the passkey, checks the handle belongs to it, verifies origin / `rpId` /
signature / counter, moves the counter forward, opens the session exactly like `login`; **every** refusal is the same
`401`, an unknown or archived account included); authenticated `GET /user/me/passkeys/` (never the key),
`POST /user/me/passkeys/options/`, `POST /user/me/passkeys/` (label + answer, `201`), `PATCH /user/me/passkeys/{id}/` (MAIR-811:
`{label}`, `200` with the passkey, no e-mail; the Login screen registers first, then saves the name the agent keeps, the manager by
default) and `DELETE /user/me/passkeys/{id}/` (own passkeys only, another account's is `404`). A passkey is a complete sign-in (it counts as both factors, so it
satisfies a mandatory policy); The name of the manager (`user_passkeys.provider`, Database 3.10.0) is deduced at registration from the AAGUID of the attestation object (`passkey_managers.rs`: CBOR `authData`, table = the community list `passkey-authenticator-aaguids`, names as published; an all-zero, absent or unknown AAGUID gives `null`) and returned as `provider` by the registration and the list, so the front can offer it as the name of the passkey. The password sign-ins (`POST /auth/login` and `/auth/login/2fa`) also answer `passkey_offer: { allowed, registered }` (not `passkey`: the GDPR contract check refuses response fields named after the credentials column `user_passkeys.passkey`) (`allowed` = relying party configured and policy accepts passkeys, `registered` = the account has one) so the login screen can propose a passkey; a failed lookup reads as nothing to propose and never blocks the sign-in, and a sign-in by passkey carries no `passkey_offer` field. `passkey_allowed = false` answers `403` on every passkey route. Adding or removing one sends
the security e-mail `account-passkey-changed` (`emails::PasskeyChangedEmail`: `USER_FIRST_NAME`, `CHANGE` = `ajoutée` /
`supprimée`, French because the template cannot branch, `PASSKEY_LABEL`, `DEVICE`, `DATE`, `IP_ADDRESS`, optional `MAIRIE_NAME`); that Resend template must be
published before deploying, and a failed e-mail never undoes the change. The sign-in shares the `login_per_ip` budget, the
authenticated routes `second_factor_per_user`. Tests use the software authenticator of `webauthn-authenticator-rs`
(`tests/endpoints/passkeys.rs`): it only signs for a listed credential and returns no user handle, so the test names the
credential and sets `userHandle` as a browser would. The newman journey cannot sign (no authenticator in the sandbox): it
covers the challenges, the refusals and the policy gate, and the `201` / `204` / `409` (and the `200` of the rename) stay in the coverage exceptions.
`load-test.js` runs the non-read passkey handlers in the isolated `policy` scenario (passkeys are refused by default).

### Settings V2 (MAIR-823)

Routes the Settings front needs (through BFF_Settings), all `jwt`-secured and acting on the caller only:

- `DELETE /api/v1/sessions/{sessionId}/` ends one active session of the caller by its UUID (`204`). The session of the request
  answers `409` (log out instead); an unknown id, another account's session or one already revoked / expired answers the
  same `404`; a path that is not a UUID `400`. `POST /api/v1/sessions/revoke-others` ends every active session but the one
  of the request (`200 {"revoked": n}`; a JWT without `sid` revokes them all). Both publish the revoked ids to the Redis
  revocation list (`session_jwt::current_session_id` reads the `sid` of the request).
- `POST /api/v1/user/me/password {current_password, new_password}` changes the caller's password (`204`). It spends
  `password_check_per_user` first (`429` + `Retry-After`, budget shared with the e-mail change of `PATCH /user/me/`), answers
  `403` to a wrong current password or an account without one, `400` to a new password outside the 8-255 rule
  (`MIN_PASSWORD_LENGTH`) or equal to the current one, and writes the hash and revokes every **other** session in one
  transaction, published after the commit (MAIR-420).
- Preferences gain `autosave`, notification settings `system_updates` and `login_alerts` (Database `mair-823-01`, nullable
  booleans like the others).
- Sign-in alert (`src/login_alert.rs`): when `login_alerts` is `true`, every new session (password login, `login/2fa`,
  passkey, `reset_password`, the two mandatory enrolments) sends the Resend template `account-login-alert`
  (`emails::LoginAlertEmail`: `USER_FIRST_NAME`, `DEVICE`, `IP_ADDRESS`, `DATE`, optional `MAIRIE_NAME`) on a background
  task: it never delays nor blocks the sign-in, and a failure is only logged. **The template must be published in Resend
  before deploying.** Tests: `tests/endpoints/mair_823_settings_v2.rs`, newman folder "Settings V2: sessions, password,
  alerts (MAIR-823)".

### Welcome tour state (MAIR-1176)

`GET` / `PATCH /api/v1/user/me/preferences/` carry `onboarding_completed` (Database `mair-1176-01`,
`user_preferences.onboarding_completed`, NULL read as `false`): `true` once the caller finished or skipped the Dashboard
welcome tour, `false` while not done (new account, or after a `null` reset), never `null`. The `PATCH` accepts `true` or `null` and answers `400`
(plain text naming the field) to `false`; like every preference, a key left out keeps its value. BFF_Dashboard reads it in
its bootstrap and writes `true` from `PUT /dashboard/onboarding`. Tests: `tests/endpoints/user_preferences.rs`
(`mair_1176_*`), the unit test of `PatchPreferencesView`, newman folder "Welcome tour state (MAIR-1176)".

### Stolen sessions (MAIR-773)

The APIs cannot see the browser: the fronts' Next.js proxies compare each request with its session (device proof,
network) and report what they found with the session's own JWT, through BFF_user, to `POST /api/v1/sessions/report
{level, ip_address, device_info}` (`level`: `certain` = bound session without a valid proof, `possible` = unbound
session from another network, `ip_change` = bound session that changed network with its proof). It calls Database's
`report_suspicious_session_use()` (Database `mair-773-01`): every report is logged in `connection_logs`,
`certain` / `possible` revoke the session (`revoked: true`, published to Redis like any revocation) and queue one alert
per session and level in `session_security_alerts`, `ip_change` moves the session's address. A JWT without `sid`
answers `400`. `src/session_alerts.rs` e-mails each alert to the agent (Resend
template `account-session-alert`: `USER_FIRST_NAME`, `CERTAINTY` = `certaine` / `possible`, `DEVICE`, `IP_ADDRESS`
(only its beginning, `92.184.xxx.xxx`, `emails::mask_ip_address`), `DATE`, `CHANGE_PASSWORD_URL` =
`<PUBLIC_URL>/forgot-password`, optional `MAIRIE_NAME`) and to every active administrator (`admin-session-alert`:
`AGENT_NAME` instead of the first name, no `CHANGE_PASSWORD_URL`, and `JOURNAL_URL` =
`<ADMIN_PUBLIC_URL>/?tab=security&journal=<user id>` when the optional `ADMIN_PUBLIC_URL` is set), right after the report and from a sweep every minute started by `main` (`spawn_sweeper`): each replica
claims its alerts with `FOR UPDATE SKIP LOCKED`, 5 attempts one minute apart, the failure kind in `last_error`.
**Both templates must be published in Resend before deploying.** The Login screen reads the latest unread alert with
`GET /api/v1/user/me/security-alert/` (`204` when none) and marks it read with `POST
/api/v1/user/me/security-alert/{alertId}/acknowledge/`; the administration reads `GET /api/v1/admin/security-alerts/`
(30 days, 500 at most, with the agent) and `GET /api/v1/admin/users/{userId}/connection-logs` (latest 100 lines).
Tests: `tests/endpoints/mair_773_session_alerts.rs`, newman folder "Stolen session alerts (MAIR-773)".

### Sessions / auth flow

Login (`endpoints/v1/auth/login/endpoint.rs`) always checks the password first (archived and
passwordless accounts answer the same `401`, after the same argon2 work). On a user's first connection a
**correct** password returns `412 Precondition Failed` with a one-time token (stored in Redis) instead of
logging in, forcing a password change via `force_change_password`. On success it issues a JWT
(`session_jwt::generate_session_jwt`) plus an opaque refresh token: only its SHA-256 digest is stored in
`sessions.token_hash` (`refresh_token::hash`), and `POST /sessions/refresh` rotates it (the JSON body returns
the new token, the old one stops working). Hash any refresh token before handing it to a session query view.
Database keeps every hash a rotation replaced (`session_replaced_refresh_tokens`, filled by a trigger, until the
session ends). A refresh token that matches no current token goes to Database's `report_refresh_token_reuse()`
(`database::sessions::report_refresh_token_reuse`) with the client address and the device (`device_info` of the body,
the request's `User-Agent` otherwise). Replaced less than `REFRESH_REUSE_GRACE_SECONDS` ago (10 s on the Postgres clock)
while the session is active, it is a concurrent renewal by another front: `409` `refresh_superseded`, nothing changes,
and the front reads the cookie the other renewal set. Later, Database logs `REFRESH_REUSE` in `connection_logs`, revokes
the session and queues one `certain` alert (e-mailed like the stolen-session alerts below); the refresh handler
publishes the session to the Redis revocation list (retried 3 times, then a `[CRITICAL]` audit log), logs an `audit`
event (`refresh_token_reuse`, never the token), and the answer is the same `401` as for an unknown token. The rotation
still writes `previous_token_hash` / `rotated_at` for the Core release they replace.

Session state (MAIR-772, `mairie360_api_lib` 5.1.0): every JWT Core issues (`login::finish_session`, so every sign-in path,
and the refresh) is published as the active token of its session with `session_jwt::publish_active_token`
(`session:<sid>` = SHA-256 of the JWT, user id, address; TTL `JWT_TIMEOUT` clamped to 15 minutes by the lib), and
`session_revocation::publish_revoked_session` writes the `revoked` state with `revoked:<sid>` (`mark_session_revoked`).
The lib's middleware then accepts the current JWT of a session without any database query, refuses the JWT a refresh
replaced (`401 Replaced token`) and, when the key is missing, asks Database's `session_state()` and caches the answer. A
failed publication fails the sign-in (`500`) or the refresh (`503`, the rotation is rolled back, the refresh token sent
stays valid): a JWT Redis does not know would be refused as replaced. With a `JWT_TIMEOUT` above 15 minutes the
replaced token is only refused while the key lives. Every API role needs `~session:*` in the chart's Redis ACL
(`tests/common/acl_redis.rs` mirrors it). Tests: `tests/endpoints/mair_772_session_state.rs`.

The JWT claims (`session_jwt::SessionClaims`) are `sub`, `exp`, `sid` (session id, MAIR-226) and, since MAIR-529,
`role` (the user's role, one per user since MAIR-564, empty string when none); the `EdDSA` tokens (MAIR-680) also
carry `iss` and `aud`.
`session_jwt::user_role_name` reads it at every login (password, reset password) and every refresh,
so a role change reaches the JWT at the next refresh only. The BFFs read the role from the token (`bffs_lib`
`sessionRole`) instead of calling `/user/me/`; JWTs issued before MAIR-529 have an empty `role`. There is no
`roles` claim any more (tokens signed with one still decode: unknown claims are ignored).

Argon2 (MAIR-474): hash and verify passwords through `crate::passwords::{hash, verify}`, which run on actix's
blocking pool (`web::block`), never by calling `mairie360_api_lib::password::{hash_password, verify_password}` from a
handler: inline, each hash blocks the worker thread and every request queued on it.

Security knobs added by MAIR-390, all optional:

- `TRUSTED_PROXIES` (`client_ip.rs`): comma-separated addresses / CIDR ranges whose `X-Forwarded-For` /
  `Forwarded` is trusted; loopback + private ranges by default, empty string = trust none. Read client
  addresses with `client_ip::client_ip`, never `realip_remote_addr()`.
- `RATE_LIMIT_ENABLED=false` (`rate_limit.rs`) turns off the in-memory per-replica budgets of login,
  forgot/reset/force_change_password, refresh and the current-password check of
  `PATCH /user/me/` (per user, MAIR-746) (`429` + `Retry-After`). A new route that checks a password
  spends `password_check_per_user` before comparing the hash. Handlers take
  `Option<web::Data<RateLimits>>`; tests that do not register it are not limited.

### Database schema

Postgres schema is **not** managed in this repo — it lives in a separate Liquibase migrations image
(`ghcr.io/mairie360/liquibase-migrations`, run as the `liquibase` service in `docker-compose.yml`) applied
against the `ghcr.io/mairie360/database` image; its source is `../../Devops/Database`.

### Docs referenced in this repo

- Swagger UI at `/swagger-ui/` (when `SWAGGER_ENABLED=true`) and `cargo open_api` are the endpoint reference; the stale `API.md` was removed (MAIR-245).

### Logs and observability (MAIR-131, MAIR-503)

Logs and traces come from `mairie360_api_lib::logging::telemetry` (feature `otel`), shared by every
API since MAIR-503 (Core hosted the original POC of MAIR-131 in `src/telemetry.rs`, now removed):
`main.rs` only calls `init_telemetry("core-api")` and keeps the guard. Read the lib for the details.

- Logs on stdout in the format of every API: coloured text (`RUST_LOG`, default `info`), one JSON
  object per line with `LOG_FORMAT=json`. They are always on (MAIR-421): the handlers report their
  database errors through `tracing`, export or not. The request root span is hidden from them (it
  holds the client address and the query string, MAIR-290).
- Traces over OTLP/HTTP (protobuf), opt-in: nothing is exported unless
  `OTEL_EXPORTER_OTLP_ENDPOINT` (or `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`) is set,
  `OTEL_SDK_DISABLED=true` forces it off, a failing exporter is logged and never stops the API. The
  endpoint is an agent relaying to Scaleway Cockpit (OTel Collector or Grafana Alloy, e.g.
  `http://alloy:4318`, `/v1/traces` is appended); `OTEL_SERVICE_NAME` defaults to `core-api`,
  `OTEL_EXPORTER_OTLP_HEADERS` carries a token when the endpoint needs one.
- `main.rs` wraps the app in the lib's `TracingLogger` (outermost, so requests refused by the JWT /
  session guards get a span too; there is no `middleware::Logger`). Root spans are named
  `<METHOD> <route pattern>`, carry `http.*` attributes and continue an incoming `traceparent`.
- No personal data leaves in a span: the lib's `Redact` span processor drops `http.client_ip` and
  the query string of `http.target` (directory search terms) before the export. Build test
  providers with `telemetry::tracer_provider(processor, "core-api")`, never
  `SdkTracerProvider::builder()`, so the redaction always applies.
- The SQL of `mairie360_api_lib` (sqlx) appears as span **events** (`db.statement` with `$n`
  placeholders, never the bound values, plus `elapsed` and the row counts), not as child spans:
  sqlx reports statements as `tracing` events of the `sqlx::query` target. Real `db` spans need a
  change in `API_lib`.
- Never add `opentelemetry*`, `tracing-opentelemetry` or `tracing-actix-web` to `Cargo.toml`: use
  the lib's re-exports (`logging::telemetry::{opentelemetry, opentelemetry_sdk, TracingLogger}`),
  their versions are coupled and bumped in the lib. `tracing-subscriber` is only a dev-dependency
  (the telemetry tests build their own registry).
- CPU/RAM of the pod are not app metrics: they come from the cluster agent, not from this crate.
- `tests/endpoints/telemetry.rs` asserts, against an in-memory exporter, the span of a real route
  (status, route, continued trace id, SQL events, neither client address nor query string) and of a
  `401` (Docker needed like the other integration tests); the redaction of logs and spans and
  `init_telemetry` are tested in the lib.

### Erasure and export (MAIR-289)

`POST /api/v1/admin/users/{userId}/erase` (admin) erases an account for good, unlike the archiving
of `DELETE /api/v1/admin/users/{userId}/`: it calls `anonymize_user()` of Devops/Database (MAIR-289,
`SECURITY DEFINER`, granted to `core_api`), which applies the `erasure` of `gdpr/inventory.yaml`
(identity cleared, memberships and sessions deleted, private events deleted, groups / projects /
events handed over to an active admin, audit identity hashed), then publishes the returned
`revoked_sessions` to the revocation list. SQLSTATE `P0002` → 404, `23001` (seeded admin, or no
other active admin to take the objects over) → 409. `GET /api/v1/user/me/export` and
`GET /api/v1/admin/users/{userId}/export` return `export_user_data()` as is (`src/endpoints/export_data.rs`).
The views are in `src/database/users/erasure/`. `tests/endpoints/erasure.rs` runs against the
`TEST_DB_VERSION` image (`dev`, MAIR-998; the functions ship with Database #166).

### Access matrix (MAIR-288)

`access-matrix.yaml` (root) is the access decision of every operation of the OpenAPI: `access`
(`public` / `authenticated` / `admin`), the `roles` that pass whatever the instance, the
`ownership` that also passes (`self`, `owner` = owner or ACL, `member`), the `personal` fields of
the answer, `token: true` when the body carries a credential. `tests/endpoints/access_matrix.rs`
fails when an operation of `ApiDoc` is missing from the file (or the file names one that does not
exist), then calls every operation as a user of each role, as the owner of the instance and
without a session, on fresh fixtures: an allowed caller never gets 401 / 403, any other gets 403.
A new endpoint therefore needs its line in the matrix, decided with the mairie. Decisions recorded
there: the phone is visible to every agent (`GET /api/v1/user/{id}/`), an archived account only
to itself and the administrators; `DIRECTORY_GUEST_ACCESS=none` (read on every call, default
`all`) closes `GET /api/v1/user/` and the other users' records to the agents who only hold the
Guest role (`src/endpoints/v1/user/directory_access.rs`).

## CI

`.github/workflows/cicd.yml` delegates to the reusable `mairie360/CICD` workflow (`APIs_cicd.yml`) on every
push, with no input other than the package name and the CICD version. The `uses:` is pinned by release tag (the org
Renovate preset sets `pinDigests: false`; no SHA pin, `tests/mair_685_auto_approve.rs` checks the tag), `cicd_version` carries the same tag, and Renovate bumps both in one `mairie360/CICD` group; only
the two secrets the workflow declares are passed (no `secrets: inherit`). Both Dockerfiles build on the template's
Rust toolchain pinned by digest, with `--locked` and a dependency-cache layer (MAIR-427); the staging gates of `promote-staging.yml` (MAIR-494) run the three `*_test.sh` scripts with `IMAGE_REF` set to the `dev-<sha>` image published by `release-dev` (newman, no Postman account involved). `auto-approve.yml` calls the reusable auto-approval of mairie360/CICD (same tag on `uses:` and `cicd_version`), which approves a Renovate PR only while Renovate alone wrote and pushed it and takes the approval back after any other push; `pull-requests: write` only (`tests/mair_685_auto_approve.rs`).

## Pull request reviewers

Every PR requests a review from the whole team, minus its author: `CarolinHugo`, `LAURETbenjamin`, `MathTek` and `Quentintnrl` (`gh pr create … --reviewer CarolinHugo,LAURETbenjamin,MathTek`). `.github/CODEOWNERS` makes GitHub request them automatically as well.

## Postgres pool settings (MAIR-565)

Since `mairie360_api_lib` 3.4.0, `Database` reads three optional env vars through `DbPoolConfig::from_env`:
`DB_POOL_MIN_CONNECTIONS` (default 1, sqlx's was 0: the API keeps one connection open, so the first request after a
pause no longer reconnects), `DB_POOL_MAX_CONNECTIONS` (default 10) and `DB_POOL_IDLE_TIMEOUT_SECS` (default 600). An
invalid value falls back to its default with a `warn!`. Per-instance values are set by the `Deploiment` chart
(`APIs.dbPool`); the compose stacks set none and use the defaults.

## Promotion to staging and prod (MAIR-494)

A push to `main` only releases dev (`cicd.yml` stops at `release-dev`). Staging and prod are manual:
`gh workflow run promote-staging.yml -R mairie360/Core_API [-f sha=<sha>]`, then `promote-prod.yml`
(environments `Staging` / `Prod`). Staging refuses a commit without `dev-<sha>` and a green main run, and runs
the integration, ZAP and k6 stacks on that image; prod refuses one without `staging-<sha>` and tags the version semantic-release computes
for that commit. `cicd.yml`, `promote-staging.yml` and `promote-prod.yml` pin the same mairie360/CICD release.
