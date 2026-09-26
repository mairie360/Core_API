use actix_web::{http::StatusCode, test, web, App};
use core_api::database::auth::register::RegisterUserQueryView;
use core_api::database::get_user_id::GetUserIdQueryView;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::{
    security::JwtMiddleware, state::AppState, test_setup::queries_setup::get_shared_db,
};
use serde_json::{json, Value};
use serial_test::serial;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "preferences_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

const PREFERENCES: &str = "/api/v1/user/me/preferences/";
const NOTIFICATIONS: &str = "/api/v1/user/me/notifications/";

/// Same mounting as `main.rs`: public routes, then the `/api` scope behind `JwtMiddleware`.
macro_rules! init_app {
    ($state:expr) => {
        test::init_service(
            App::new()
                .app_data($state.clone())
                .configure(public_config)
                .service(web::scope("/api").wrap(JwtMiddleware).configure(config)),
        )
        .await
    };
}

/// Registers a fresh user and returns a JWT for it.
async fn fresh_user_jwt(state: &web::Data<AppState>) -> String {
    let email = format!("preferences_endpoint_{}@example.com", uuid::Uuid::new_v4());
    let db = state.get_smart_db();
    let _: bool = db
        .fetch_scalar(&RegisterUserQueryView::new(
            "Prefs", "Endpoint", &email, "password", None,
        ))
        .await
        .unwrap();
    let user_id: i32 = db
        .fetch_scalar(&GetUserIdQueryView::new(&email))
        .await
        .unwrap();
    generate_jwt(&user_id.to_string(), "User").unwrap()
}

#[tokio::test]
#[serial]
async fn preferences_require_a_jwt() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let app = init_app!(state);

    for uri in [PREFERENCES, NOTIFICATIONS] {
        // The middleware answers with an error, turned into the 401 response by actix.
        let error = test::try_call_service(&app, test::TestRequest::get().uri(uri).to_request())
            .await
            .expect_err(uri);
        assert_eq!(
            error.as_response_error().status_code(),
            StatusCode::UNAUTHORIZED,
            "{uri}"
        );
    }
}

#[tokio::test]
#[serial]
async fn preferences_round_trip() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let app = init_app!(state);
    let jwt = fresh_user_jwt(&state).await;
    let bearer = ("Authorization", format!("Bearer {jwt}"));

    let req = test::TestRequest::get()
        .uri(PREFERENCES)
        .insert_header(bearer.clone())
        .to_request();
    let body: Value = test::call_and_read_body_json(&app, req).await;
    assert!(
        body.as_object().unwrap().values().all(Value::is_null),
        "{body}"
    );

    let req = test::TestRequest::patch()
        .uri(PREFERENCES)
        .insert_header(bearer.clone())
        .set_json(json!({ "theme": "dark", "font_size": 16, "language": "fr" }))
        .to_request();
    let body: Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(body["theme"], "dark");
    assert_eq!(body["font_size"], 16);

    // Absent keeps, `null` resets.
    let req = test::TestRequest::patch()
        .uri(PREFERENCES)
        .insert_header(bearer.clone())
        .set_json(json!({ "font_size": null, "timezone": "Europe/Paris" }))
        .to_request();
    let body: Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(body["theme"], "dark");
    assert_eq!(body["language"], "fr");
    assert_eq!(body["timezone"], "Europe/Paris");
    assert!(body["font_size"].is_null());

    let req = test::TestRequest::get()
        .uri(PREFERENCES)
        .insert_header(bearer)
        .to_request();
    let read: Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(read, body);
}

#[tokio::test]
#[serial]
async fn invalid_preferences_answer_400() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let app = init_app!(state);
    let jwt = fresh_user_jwt(&state).await;
    let bearer = ("Authorization", format!("Bearer {jwt}"));

    for (uri, body) in [
        (PREFERENCES, json!({ "theme": "neon" })),
        (PREFERENCES, json!({ "font_size": 0 })),
        (PREFERENCES, json!({ "language": "a".repeat(17) })),
        (PREFERENCES, json!({ "home_page": "<script>" })),
        (PREFERENCES, json!({ "timezone": "  " })),
        (NOTIFICATIONS, json!({ "email": "yes" })),
    ] {
        let req = test::TestRequest::patch()
            .uri(uri)
            .insert_header(bearer.clone())
            .set_json(&body)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{uri} {body}");
    }
}

#[tokio::test]
#[serial]
async fn notification_settings_round_trip() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let app = init_app!(state);
    let jwt = fresh_user_jwt(&state).await;
    let bearer = ("Authorization", format!("Bearer {jwt}"));

    let req = test::TestRequest::patch()
        .uri(NOTIFICATIONS)
        .insert_header(bearer.clone())
        .set_json(json!({ "email": true, "push": false }))
        .to_request();
    let body: Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(
        body,
        json!({ "email": true, "push": false, "desktop": null, "messages": null, "projects": null, "calendar": null })
    );

    let req = test::TestRequest::get()
        .uri(NOTIFICATIONS)
        .insert_header(bearer)
        .to_request();
    let read: Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(read, body);
}
