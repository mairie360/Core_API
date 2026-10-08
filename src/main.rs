// Voir lib.rs : versions multiples de dépendances transitives hors de notre contrôle.
#![allow(clippy::multiple_crate_versions)]

use actix_web::{middleware, web, App, HttpServer};

use core_api::database::pg_url::build_pg_url;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::swagger::{is_swagger_enabled, ApiDoc};
use core_api::endpoints::{config, public_config};
use core_api::endpoints::{health, ready};
use core_api::keycloak::{KeycloakAdminClient, KeycloakClient, KeycloakConfig};
use core_api::rate_limit::RateLimits;
use core_api::request_log::{hide_query, restore_query, RedactedRootSpanBuilder};
use core_api::telemetry;
use mairie360_api_lib::security::JwtMiddleware;

use mairie360_api_lib::env_manager::{get_critical_env_var, get_env_var};
use mairie360_api_lib::state::AppState;
use std::time::{Duration, Instant};
use tracing_actix_web::TracingLogger;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// How long `main` waits for Postgres before giving up, overridable with
/// `DB_STARTUP_TIMEOUT_SECONDS`.
const DEFAULT_DB_STARTUP_TIMEOUT: Duration = Duration::from_secs(60);

/// Waits until Postgres answers, at most `DB_STARTUP_TIMEOUT_SECONDS` (60 by default).
///
/// # Errors
///
/// When Postgres still does not answer once the timeout is over: the process then exits with an
/// error, and the orchestrator restarts it.
async fn wait_for_postgres(state: &AppState) -> std::io::Result<()> {
    let timeout = get_env_var("DB_STARTUP_TIMEOUT_SECONDS")
        .and_then(|value| value.trim().parse().ok())
        .map_or(DEFAULT_DB_STARTUP_TIMEOUT, Duration::from_secs);
    let deadline = Instant::now() + timeout;
    loop {
        if ready::postgres_ready(state).await {
            return Ok(());
        }
        if Instant::now() >= deadline {
            tracing::error!("Postgres did not answer within {timeout:?}: refusing to start.");
            return Err(std::io::Error::other("Postgres is unreachable"));
        }
        tracing::warn!("Waiting for Postgres...");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

//                                        -- MAIN FUNCTION --

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Exports the traces when `OTEL_EXPORTER_OTLP_ENDPOINT` is set (MAIR-131); flushed on drop.
    let _telemetry = telemetry::init();
    let redis_url = get_critical_env_var("REDIS_URL");
    let db_user = get_critical_env_var("DB_USER");
    let db_password = get_critical_env_var("DB_PASSWORD");
    let db_host = get_critical_env_var("DB_HOST");
    let db_port = get_critical_env_var("DB_PORT");
    let db_name = get_critical_env_var("DB_NAME");
    let pg_url = build_pg_url(&db_user, &db_password, &db_host, &db_port, &db_name);
    let state = AppState::new(redis_url, pg_url).await;
    // The lib starts without a database (its pool stays empty): refuse to serve rather than
    // answer every request with a 500 (MAIR-423).
    wait_for_postgres(&state).await?;
    let data = web::Data::new(state);
    // Keycloak sign-in is optional during the transition: without its env vars, only the
    // password login is available and `POST /api/v1/auth/keycloak` answers 503.
    let keycloak_config = KeycloakConfig::from_env();
    let keycloak = keycloak_config
        .clone()
        .map(|config| web::Data::new(KeycloakClient::new(config)));
    // The administration endpoints mirror the accounts they change into the realm through the
    // Admin API (MAIR-142), which needs a confidential client: with a public client (or without
    // Keycloak) they only write to Core.
    let keycloak_admin = keycloak_config
        .map(KeycloakAdminClient::new)
        .filter(|admin| {
            if admin.is_configured() {
                true
            } else {
                tracing::warn!(
                    "Keycloak account synchronisation disabled: KEYCLOAK_CLIENT_SECRET is not set or KEYCLOAK_REALM_URL has no /realms/ segment."
                );
                false
            }
        })
        .map(web::Data::new);
    // Budgets of the public authentication routes (MAIR-390), shared by every worker of this
    // replica. `RATE_LIMIT_ENABLED=false` turns them off (load tests).
    let rate_limits = RateLimits::from_env().map(web::Data::new);
    // Swagger UI and the spec are only served where SWAGGER_ENABLED=true (MAIR-424).
    let swagger = is_swagger_enabled(get_env_var).then(ApiDoc::openapi);
    let host = get_critical_env_var("HOST");
    let port = get_critical_env_var("PORT");
    let bind_address = format!("{host}:{port}");
    let server = HttpServer::new(move || {
        let app = App::new().app_data(data.clone());
        let app = match &keycloak {
            Some(keycloak) => app.app_data(keycloak.clone()),
            None => app,
        };
        let app = match &keycloak_admin {
            Some(admin) => app.app_data(admin.clone()),
            None => app,
        };
        let app = match &rate_limits {
            Some(limits) => app.app_data(limits.clone()),
            None => app,
        };
        let app = match &swagger {
            Some(spec) => app.service(
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", spec.clone()),
            ),
            None => app,
        };
        // One root span per request, including the ones the JWT / session guards refuse. It
        // records the path without the query string and no error `Debug` (MAIR-290, see
        // `request_log`): `hide_query` runs before it, `restore_query` right after it.
        app.wrap(middleware::from_fn(restore_query))
            .wrap(TracingLogger::<RedactedRootSpanBuilder>::new())
            .wrap(middleware::from_fn(hide_query))
            // Every response is JSON or plain text: forbid browsers from sniffing it as HTML.
            .wrap(middleware::DefaultHeaders::new().add(("X-Content-Type-Options", "nosniff")))
            .service(health::health)
            .service(ready::ready)
            // Routes /api publiques (refresh du JWT) : avant le scope protégé, qui sinon les capte
            .configure(public_config)
            // 3. Endpoints Protégés par JWT
            // The last `wrap` runs first: JwtMiddleware validates the JWT, then session_guard
            // rejects it if its session was revoked (MAIR-226).
            .service(
                web::scope("/api")
                    .wrap(middleware::from_fn(session_guard))
                    .wrap(JwtMiddleware)
                    .configure(config),
            )
    })
    .bind(bind_address)?;

    let addr = server.addrs().first().copied();
    tokio::spawn(async move {
        if let Some(addr) = addr {
            tracing::info!("Server listening on http://{addr}");
        }
    });

    server.run().await
}
