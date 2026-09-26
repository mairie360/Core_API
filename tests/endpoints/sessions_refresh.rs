use actix_web::{http::StatusCode, test, web, App};
use core_api::database::sessions::{
    create_session::CreateSessionQueryView, revoke_session_by_token::RevokeSessionByTokenQueryView,
};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config, v1::sessions::REFRESH_PATH};
use mairie360_api_lib::{
    security::JwtMiddleware, state::AppState, test_setup::queries_setup::get_shared_db,
};
use serde_json::json;
use serial_test::serial;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

/// Même montage que `main.rs` : routes publiques puis scope `/api` protégé par `JwtMiddleware`.
macro_rules! init_app {
    ($state:expr) => {
        test::init_service(
            App::new()
                .app_data($state.clone())
                .configure(public_config)
                .service(
                    web::scope("/api")
                        .wrap(actix_web::middleware::from_fn(session_guard))
                        .wrap(JwtMiddleware)
                        .configure(config),
                ),
        )
        .await
    };
}

async fn create_session(state: &web::Data<AppState>, token: &str) {
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::new(
            1,
            token,
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();
}

#[tokio::test]
#[serial]
async fn test_refresh_works_with_expired_jwt() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let app = init_app!(state);

    let token = format!("refresh_expired_jwt_{}", uuid::Uuid::new_v4());
    create_session(&state, &token).await;

    // JWT expiré/invalide : le middleware l'aurait rejeté si le refresh était sous le scope `/api`.
    let req = test::TestRequest::post()
        .uri(REFRESH_PATH)
        .insert_header(("Authorization", "Bearer expired.jwt.token"))
        .set_json(json!({ "refresh_token": token }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let authorization = resp
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    assert!(authorization.starts_with("Bearer "), "{authorization}");
}

#[tokio::test]
#[serial]
async fn test_refresh_revoked_token_returns_401() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let app = init_app!(state);

    let token = format!("refresh_revoked_{}", uuid::Uuid::new_v4());
    create_session(&state, &token).await;
    state
        .get_smart_db()
        .execute(RevokeSessionByTokenQueryView::new(1, &token))
        .await
        .unwrap();

    let req = test::TestRequest::post()
        .uri(REFRESH_PATH)
        .set_json(json!({ "refresh_token": token }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn test_refresh_unknown_token_returns_401() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let app = init_app!(state);

    let req = test::TestRequest::post()
        .uri(REFRESH_PATH)
        .set_json(json!({ "refresh_token": "unknown_refresh_token" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn test_other_session_routes_still_require_jwt() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let app = init_app!(state);

    let req = test::TestRequest::post()
        .uri("/api/v1/sessions/revoke")
        .set_json(json!({ "refresh_token": "any" }))
        .to_request();
    let status = match test::try_call_service(&app, req).await {
        Ok(resp) => resp.status(),
        Err(err) => err.as_response_error().status_code(),
    };

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
