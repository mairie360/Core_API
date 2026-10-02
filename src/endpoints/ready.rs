//! Readiness probe (MAIR-423).
//!
//! `/health` only tells that the process accepts connections (liveness: restart it when it stops
//! answering). `/ready` also checks the dependencies every route needs, Postgres and Redis, so
//! Kubernetes takes a replica that lost one of them out of the service instead of letting it
//! serve `500`s.

use actix_web::{get, web, HttpResponse, Responder};
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use mairie360_api_lib::state::AppState;
use std::time::Duration;
use utoipa::OpenApi;

/// Longest a dependency check may take before the dependency counts as down.
const CHECK_TIMEOUT: Duration = Duration::from_secs(2);

/// Key read by the Redis check. It never exists: only the round trip matters, and it lives under
/// the replica's key prefix, which the Redis ACL of the chart allows.
const REDIS_PROBE_KEY: &str = "readiness-probe";

/// `SELECT 1`: a round trip to Postgres through the pool.
#[derive(serde::Deserialize)]
pub struct PingQueryView;

impl ApiRequestDto for PingQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT 1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &[]
    }
}

/// Whether Postgres answers within [`CHECK_TIMEOUT`].
pub async fn postgres_ready(state: &AppState) -> bool {
    matches!(
        tokio::time::timeout(
            CHECK_TIMEOUT,
            state.get_smart_db().fetch_scalar::<i32, _>(&PingQueryView),
        )
        .await,
        Ok(Ok(1))
    )
}

/// Whether Redis answers within [`CHECK_TIMEOUT`].
pub async fn redis_ready(state: &AppState) -> bool {
    matches!(
        tokio::time::timeout(CHECK_TIMEOUT, state.get_redis().key_exist(REDIS_PROBE_KEY)).await,
        Ok(Ok(_))
    )
}

#[utoipa::path(
    get,
    path = "ready",
    summary = "Readiness probe",
    description = "Checks that the replica can serve requests: Postgres answers a `SELECT 1` and \
                   Redis answers a command, each within 2 seconds. Unauthenticated, used as the \
                   Kubernetes readiness probe and by the Docker stacks to wait for the API.\n\n\
                   Unlike `/health` (liveness, which only tells that the process accepts \
                   connections), a `503` here does not mean the process must be restarted: the \
                   replica is taken out of the service until its dependencies come back.",
    responses(
        (
            status = 200,
            description = "Postgres and Redis both answered.",
            body = String,
            content_type = "text/plain",
            example = json!("OK")
        ),
        (
            status = 503,
            description = "Postgres or Redis did not answer in time; the body names the ones that failed.",
            body = String,
            content_type = "text/plain",
            example = json!("Not ready: postgres")
        )
    ),
    tag = "Service"
)]
#[get("/ready")]
pub async fn ready(state: web::Data<AppState>) -> impl Responder {
    let (postgres, redis) = tokio::join!(postgres_ready(&state), redis_ready(&state));
    let failed: Vec<&str> = [("postgres", postgres), ("redis", redis)]
        .into_iter()
        .filter_map(|(name, ok)| (!ok).then_some(name))
        .collect();
    if failed.is_empty() {
        HttpResponse::Ok().content_type("text/plain").body("OK")
    } else {
        tracing::warn!(failed = ?failed, "readiness probe failed");
        HttpResponse::ServiceUnavailable()
            .content_type("text/plain")
            .body(format!("Not ready: {}", failed.join(", ")))
    }
}

#[derive(OpenApi)]
#[openapi(paths(ready))]
pub struct ReadyDoc;
