use crate::common::core_db_url;
use actix_web::{http::StatusCode, test, web, App};
use core_api::database::auth::register::RegisterUserQueryView;
use core_api::database::get_user_id::GetUserIdQueryView;
use core_api::endpoints::v1::user::me::preferences::view::{
    DATE_FORMATS, DENSITIES, FONT_FAMILIES,
};
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::seed_password_hash;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    mairie360_api_lib::test_setup::http_setup::init_test_jwt_env();
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
            "Prefs",
            "Endpoint",
            &email,
            seed_password_hash(),
            None,
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
async fn preferences_require_a_jwt() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
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
async fn preferences_round_trip() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
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
async fn invalid_preferences_answer_400() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let app = init_app!(state);
    let jwt = fresh_user_jwt(&state).await;
    let bearer = ("Authorization", format!("Bearer {jwt}"));

    for (uri, body) in [
        (PREFERENCES, json!({ "theme": "neon" })),
        (PREFERENCES, json!({ "font_size": 0 })),
        (PREFERENCES, json!({ "language": "a".repeat(17) })),
        (PREFERENCES, json!({ "home_page": "home\u{7}page" })),
        (PREFERENCES, json!({ "timezone": "  " })),
        // MAIR-479: the value ZAP's path traversal rule sends, and its payloads, on every text field.
        (PREFERENCES, json!({ "density": "general" })),
        (PREFERENCES, json!({ "date_format": "general" })),
        (PREFERENCES, json!({ "language": "general" })),
        (PREFERENCES, json!({ "font_family": "general" })),
        (PREFERENCES, json!({ "timezone": "general" })),
        (PREFERENCES, json!({ "home_page": "general" })),
        (PREFERENCES, json!({ "timezone": "../../../../etc/passwd" })),
        (PREFERENCES, json!({ "home_page": "/../../etc/passwd" })),
        (PREFERENCES, json!({ "home_page": "//evil.example" })),
        (
            PREFERENCES,
            json!({ "font_family": "c:\\Windows\\system.ini" }),
        ),
        (PREFERENCES, json!({ "language": "fr_FR" })),
        (PREFERENCES, json!({ "timezone": "europe/paris" })),
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

/// Every value the application offers fits the `user_preferences` columns (MAIR-479).
#[tokio::test]
async fn allowed_preference_values_are_stored() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let app = init_app!(state);
    let jwt = fresh_user_jwt(&state).await;
    let bearer = ("Authorization", format!("Bearer {jwt}"));

    let mut bodies = vec![json!({
        "language": "fr-FR",
        "timezone": "America/Argentina/ComodRivadavia",
        "home_page": format!("/{}", "a".repeat(127)),
    })];
    bodies.extend(FONT_FAMILIES.map(|font| json!({ "font_family": font })));
    bodies.extend(DENSITIES.map(|density| json!({ "density": density })));
    bodies.extend(DATE_FORMATS.map(|format| json!({ "date_format": format })));
    for body in bodies {
        let req = test::TestRequest::patch()
            .uri(PREFERENCES)
            .insert_header(bearer.clone())
            .set_json(&body)
            .to_request();
        let stored: Value = test::call_and_read_body_json(&app, req).await;
        for (field, value) in body.as_object().unwrap() {
            assert_eq!(&stored[field], value, "{body}");
        }
    }
}

#[tokio::test]
async fn notification_settings_round_trip() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
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
        json!({ "email": true, "push": false, "desktop": null, "messages": null, "projects": null, "calendar": null, "system_updates": null, "login_alerts": null })
    );

    let req = test::TestRequest::get()
        .uri(NOTIFICATIONS)
        .insert_header(bearer)
        .to_request();
    let read: Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(read, body);
}

/// A new account has not done the welcome tour (`null`), `true` marks it done without
/// touching the other preferences, a second `true` changes nothing and `null` resets it.
#[tokio::test]
async fn mair_1176_welcome_tour_state_round_trip() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let app = init_app!(state);
    let jwt = fresh_user_jwt(&state).await;
    let bearer = ("Authorization", format!("Bearer {jwt}"));

    let req = test::TestRequest::get()
        .uri(PREFERENCES)
        .insert_header(bearer.clone())
        .to_request();
    let fresh: Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(fresh["onboarding_completed"], Value::Null);
    assert!(fresh
        .as_object()
        .unwrap()
        .contains_key("onboarding_completed"));

    let req = test::TestRequest::patch()
        .uri(PREFERENCES)
        .insert_header(bearer.clone())
        .set_json(json!({ "theme": "dark" }))
        .to_request();
    let _: Value = test::call_and_read_body_json(&app, req).await;

    for _ in 0..2 {
        let req = test::TestRequest::patch()
            .uri(PREFERENCES)
            .insert_header(bearer.clone())
            .set_json(json!({ "onboarding_completed": true }))
            .to_request();
        let done: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(done["onboarding_completed"], json!(true));
        assert_eq!(done["theme"], json!("dark"));
    }

    let req = test::TestRequest::get()
        .uri(PREFERENCES)
        .insert_header(bearer.clone())
        .to_request();
    let read: Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(read["onboarding_completed"], json!(true));

    let req = test::TestRequest::patch()
        .uri(PREFERENCES)
        .insert_header(bearer)
        .set_json(json!({ "onboarding_completed": null }))
        .to_request();
    let reset: Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(reset["onboarding_completed"], Value::Null);
    assert_eq!(reset["theme"], json!("dark"));
}

/// `false` and a value of the wrong type are refused with a plain-text `400` and save
/// nothing; without a JWT the route answers `401`.
#[tokio::test]
async fn mair_1176_welcome_tour_state_refusals() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let app = init_app!(state);
    let jwt = fresh_user_jwt(&state).await;
    let bearer = ("Authorization", format!("Bearer {jwt}"));

    let req = test::TestRequest::patch()
        .uri(PREFERENCES)
        .insert_header(bearer.clone())
        .set_json(json!({ "onboarding_completed": false }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = String::from_utf8(test::read_body(resp).await.to_vec()).unwrap();
    assert!(body.contains("onboarding_completed"), "{body}");

    let req = test::TestRequest::patch()
        .uri(PREFERENCES)
        .insert_header(bearer.clone())
        .set_json(json!({ "onboarding_completed": "yes" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let req = test::TestRequest::patch()
        .uri(PREFERENCES)
        .set_json(json!({ "onboarding_completed": true }))
        .to_request();
    // The middleware answers with an error, turned into the 401 response by actix.
    let error = test::try_call_service(&app, req)
        .await
        .expect_err("a PATCH without JWT is refused");
    assert_eq!(
        error.as_response_error().status_code(),
        StatusCode::UNAUTHORIZED
    );

    let req = test::TestRequest::get()
        .uri(PREFERENCES)
        .insert_header(bearer)
        .to_request();
    let read: Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(read["onboarding_completed"], Value::Null);
}
