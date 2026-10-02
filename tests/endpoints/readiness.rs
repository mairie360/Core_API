//! `/ready` checks Postgres and Redis, `/health` stays a plain liveness probe (MAIR-423).

use crate::common::acl_redis::AclRedis;
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::{config, health, ready};
use mairie360_api_lib::state::AppState;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

macro_rules! init_app {
    ($state:expr) => {
        test::init_service(
            App::new()
                .app_data($state.clone())
                .service(health::health)
                .service(ready::ready)
                .service(web::scope("/api").configure(config)),
        )
        .await
    };
}

/// `(status, body)` of `GET uri`.
macro_rules! get {
    ($app:expr, $uri:expr) => {{
        let resp = test::call_service(&$app, test::TestRequest::get().uri($uri).to_request()).await;
        let status = resp.status();
        let body = test::read_body(resp).await;
        (status, String::from_utf8_lossy(&body).to_string())
    }};
}

#[tokio::test]
#[serial]
async fn ready_needs_postgres_and_redis() {
    let (_container, host) = get_shared_db().await;
    std::env::remove_var("REDIS_KEY_PREFIX");
    std::env::remove_var("REDIS_USERNAME");

    // Redis unreachable: not ready, while the liveness probe still answers.
    let state =
        web::Data::new(AppState::new("redis://127.0.0.1:1".to_string(), host.clone()).await);
    let app = init_app!(state);
    assert_eq!(
        get!(app, "/ready"),
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Not ready: redis".to_string()
        )
    );
    assert_eq!(get!(app, "/health"), (StatusCode::OK, "OK".to_string()));

    // Both reachable (Redis with the chart's ACL).
    let redis = AclRedis::start().await;
    let state = web::Data::new(AppState::new(redis.url_as("core-api"), host.clone()).await);
    let app = init_app!(state);
    assert_eq!(get!(app, "/ready"), (StatusCode::OK, "OK".to_string()));

    // The probes are no longer duplicated under `/api`.
    assert_eq!(get!(app, "/api/health").0, StatusCode::NOT_FOUND);
}
