// See lib.rs: several versions of transitive dependencies out of our control.
#![allow(clippy::multiple_crate_versions)]

use actix_web::{middleware, web, App, HttpServer};

use core_api::database::pg_url::build_pg_url;
use core_api::emails::AccountEmails;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::swagger::{is_swagger_enabled, ApiDoc};
use core_api::endpoints::{config, public_config};
use core_api::endpoints::{health, ready};
use core_api::passkeys::Passkeys;
use core_api::rate_limit::RateLimits;
use core_api::second_factor::SecretCipher;
use mairie360_api_lib::security::JwtMiddleware;

use mairie360_api_lib::env_manager::{get_critical_env_var, get_env_var};
use mairie360_api_lib::logging::telemetry::{init_telemetry, TracingLogger};
use mairie360_api_lib::request_log::{hide_query, restore_query, RedactedRootSpanBuilder};
use mairie360_api_lib::state::AppState;
use std::time::{Duration, Instant};
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
    // Logs on stdout in the format shared by every API (`RUST_LOG`, default `info`;
    // `LOG_FORMAT=json` for JSON lines), plus the trace export when `OTEL_EXPORTER_OTLP_ENDPOINT`
    // is set (MAIR-131, MAIR-503); flushed on drop.
    let _telemetry = init_telemetry("core-api");
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
    // The e-mails go through Resend templates, from no-reply@<EMAIL_DOMAIN> (MAIR-522): refuse
    // to start without them rather than failing every password reset.
    let account_emails = web::Data::new(AccountEmails::from_env().map_err(|e| {
        tracing::error!("E-mail configuration: {e}");
        std::io::Error::other(e.to_string())
    })?);
    // Stolen-session alerts (MAIR-773): the alerts that could not be sent right after their report are
    // retried every minute.
    core_api::session_alerts::spawn_sweeper(data.clone(), account_emails.clone());
    // Budgets of the public authentication routes (MAIR-390), shared by every worker of this
    // replica. `RATE_LIMIT_ENABLED=false` turns them off (load tests).
    let rate_limits = RateLimits::from_env().map(web::Data::new);
    // Key sealing the TOTP secrets (MAIR-805). Without it the second factor is off: accounts that
    // have one cannot sign in (503) rather than being opened without it.
    let second_factor_cipher = SecretCipher::from_env()
        .map_err(|e| {
            tracing::error!("TOTP_ENCRYPTION_KEY: {e}");
            std::io::Error::other(e.to_string())
        })?
        .map(web::Data::new);
    // WebAuthn relying party (MAIR-810): `WEBAUTHN_RP_ID` + `WEBAUTHN_RP_ORIGIN`. Without them the
    // passkey routes answer 503 and the policy refuses `passkey_allowed`.
    let passkeys = Passkeys::from_env()
        .map_err(|e| {
            tracing::error!("WebAuthn configuration: {e}");
            std::io::Error::other(e)
        })?
        .map(web::Data::new);
    // Swagger UI and the spec are only served where SWAGGER_ENABLED=true (MAIR-424).
    let swagger = is_swagger_enabled(get_env_var).then(ApiDoc::openapi);
    let host = get_critical_env_var("HOST");
    let port = get_critical_env_var("PORT");
    let bind_address = format!("{host}:{port}");
    let server = HttpServer::new(move || {
        let app = App::new()
            .app_data(data.clone())
            .app_data(account_emails.clone());
        let app = match &rate_limits {
            Some(limits) => app.app_data(limits.clone()),
            None => app,
        };
        let app = match &second_factor_cipher {
            Some(cipher) => app.app_data(cipher.clone()),
            None => app,
        };
        let app = match &passkeys {
            Some(passkeys) => app.app_data(passkeys.clone()),
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
            // nosniff, and text/plain on the plain-text answers (MAIR-262).
            .wrap(core_api::endpoints::headers::default_headers())
            .service(health::health)
            .service(ready::ready)
            // Public /api routes (JWT refresh): before the protected scope, which would catch them otherwise
            .configure(public_config)
            // 3. Endpoints protected by JWT
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
