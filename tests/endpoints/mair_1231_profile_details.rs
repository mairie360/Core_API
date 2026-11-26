//! MAIR-1231: the Settings profile shows the photo, the biography, the job title and the service of
//! the signed-in user. `GET /api/v1/user/me/` returns them (`has_photo` only for the photo),
//! `PATCH /api/v1/user/me/` edits the biography and the job title, `/api/v1/user/me/photo` stores,
//! serves and removes a WebP of at most 100 KiB. Needs a Database image with `mair-1231-01`.

use crate::common::core_db_url;
use crate::common::get_raw_pool;
use actix_web::{http::StatusCode, test, web, App};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::rate_limit::RateLimits;
use core_api::session_jwt::generate_session_jwt;
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::{PgPool, Row};
use uuid::Uuid;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as the other endpoint tests: they share the test binary. The tests join
    // `process_env`, since this first call writes the environment.
    mairie360_api_lib::test_setup::http_setup::init_test_jwt_env();
    std::env::set_var("JWT_TIMEOUT", "3600");
});

macro_rules! init_app {
    ($state:expr) => {
        test::init_service(
            App::new()
                .app_data($state.clone())
                .app_data(web::Data::new(RateLimits::default()))
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

/// Status and body of a request, whether the handler answered or an extractor refused it.
macro_rules! call {
    ($app:expr, $req:expr) => {
        match test::try_call_service(&$app, $req).await {
            Ok(resp) => {
                let status = resp.status();
                let content_type = resp
                    .headers()
                    .get("content-type")
                    .map(|value| value.to_str().unwrap().to_string());
                let body = test::read_body(resp).await.to_vec();
                (status, content_type, body)
            }
            Err(err) => (err.as_response_error().status_code(), None, Vec::new()),
        }
    };
}

async fn setup() -> (web::Data<AppState>, PgPool, impl std::any::Any) {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let (redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), core_db_url().await).await);
    (state, get_raw_pool(host).await, redis)
}

/// A fresh active account with an open session; returns its id and its JWT.
async fn signed_in_agent(state: &web::Data<AppState>, raw: &PgPool) -> (u64, String) {
    let id: i32 = sqlx::query(
        "INSERT INTO users (first_name, last_name, email, status, first_connect, is_archived) \
         VALUES ('Paule', 'Profil', $1, 'active', FALSE, FALSE) RETURNING id",
    )
    .bind(format!("profile.{}@example.com", Uuid::new_v4()))
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id");
    let user_id = u64::try_from(id).unwrap();
    let session_id = Uuid::new_v4();
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            user_id,
            &core_api::refresh_token::hash(&Uuid::new_v4().to_string()),
            "Firefox on Linux",
            std::net::IpAddr::from([203, 0, 113, 7]),
        ))
        .await
        .unwrap();
    (
        user_id,
        generate_session_jwt(user_id, session_id, "").unwrap(),
    )
}

fn bearer(jwt: &str) -> (&'static str, String) {
    ("Authorization", format!("Bearer {jwt}"))
}

fn get_me(jwt: &str) -> test::TestRequest {
    test::TestRequest::get()
        .uri("/api/v1/user/me/")
        .insert_header(bearer(jwt))
}

fn patch_me(jwt: &str, body: &Value) -> test::TestRequest {
    test::TestRequest::patch()
        .uri("/api/v1/user/me/")
        .insert_header(bearer(jwt))
        .set_json(body)
}

fn put_photo(jwt: &str, photo: Vec<u8>) -> test::TestRequest {
    test::TestRequest::put()
        .uri("/api/v1/user/me/photo")
        .insert_header(bearer(jwt))
        .insert_header(("Content-Type", "image/webp"))
        .set_payload(photo)
}

fn photo(jwt: &str, method: &str) -> test::TestRequest {
    let request = match method {
        "GET" => test::TestRequest::get(),
        _ => test::TestRequest::delete(),
    };
    request
        .uri("/api/v1/user/me/photo")
        .insert_header(bearer(jwt))
}

/// A WebP header followed by `len - 16` filler bytes: what Core checks is the RIFF/WEBP signature.
fn webp(len: usize) -> Vec<u8> {
    let mut image = b"RIFF\0\0\0\0WEBPVP8 ".to_vec();
    image.resize(len, 7);
    image
}

fn json_of(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap()
}

#[tokio::test]
#[serial(process_env)]
async fn mair_1231_get_me_returns_the_empty_profile_details() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (_, jwt) = signed_in_agent(&state, &raw).await;

    let (status, _, body) = call!(app, get_me(&jwt).to_request());
    assert_eq!(status, StatusCode::OK);
    let me = json_of(&body);
    assert_eq!(me["biography"], Value::Null);
    assert_eq!(me["job_title"], Value::Null);
    assert_eq!(me["has_photo"], json!(false));
    assert_eq!(me["service"], Value::Null);
}

#[tokio::test]
#[serial(process_env)]
async fn mair_1231_patch_me_sets_and_clears_the_biography_and_the_job_title() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (_, jwt) = signed_in_agent(&state, &raw).await;

    let body = json!({
        "biography": "  Agent du service voirie.\nDepuis 2018.  ",
        "job_title": " Responsable voirie "
    });
    let (status, _, _) = call!(app, patch_me(&jwt, &body).to_request());
    assert_eq!(status, StatusCode::OK);
    let me = json_of(&call!(app, get_me(&jwt).to_request()).2);
    assert_eq!(
        me["biography"],
        json!("Agent du service voirie.\nDepuis 2018.")
    );
    assert_eq!(me["job_title"], json!("Responsable voirie"));

    // Another field alone keeps them.
    let (status, _, _) = call!(
        app,
        patch_me(&jwt, &json!({ "first_name": "Paula" })).to_request()
    );
    assert_eq!(status, StatusCode::OK);
    let me = json_of(&call!(app, get_me(&jwt).to_request()).2);
    assert_eq!(me["job_title"], json!("Responsable voirie"));

    // `null` and a blank text remove them.
    let body = json!({ "biography": null, "job_title": "   " });
    let (status, _, _) = call!(app, patch_me(&jwt, &body).to_request());
    assert_eq!(status, StatusCode::OK);
    let me = json_of(&call!(app, get_me(&jwt).to_request()).2);
    assert_eq!(me["biography"], Value::Null);
    assert_eq!(me["job_title"], Value::Null);
}

#[tokio::test]
#[serial(process_env)]
async fn mair_1231_patch_me_refuses_a_too_long_or_control_character_text() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (_, jwt) = signed_in_agent(&state, &raw).await;

    for (body, field) in [
        (json!({ "job_title": "a".repeat(101) }), "job_title"),
        (json!({ "job_title": "Chef\nde service" }), "job_title"),
        (json!({ "biography": "a".repeat(1001) }), "biography"),
        (json!({ "biography": "Bio\u{0007}" }), "biography"),
    ] {
        let (status, _, answer) = call!(app, patch_me(&jwt, &body).to_request());
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert!(String::from_utf8(answer).unwrap().contains(field), "{body}");
    }
    // Nothing was written.
    let me = json_of(&call!(app, get_me(&jwt).to_request()).2);
    assert_eq!(me["job_title"], Value::Null);
    assert_eq!(me["biography"], Value::Null);
}

#[tokio::test]
#[serial(process_env)]
async fn mair_1231_photo_is_stored_served_and_removed() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (_, jwt) = signed_in_agent(&state, &raw).await;

    let (status, _, _) = call!(app, photo(&jwt, "GET").to_request());
    assert_eq!(status, StatusCode::NOT_FOUND);

    let image = webp(4096);
    let (status, _, _) = call!(app, put_photo(&jwt, image.clone()).to_request());
    assert_eq!(status, StatusCode::NO_CONTENT);
    let me = json_of(&call!(app, get_me(&jwt).to_request()).2);
    assert_eq!(me["has_photo"], json!(true));
    let (status, content_type, body) = call!(app, photo(&jwt, "GET").to_request());
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("image/webp"));
    assert_eq!(body, image, "the photo comes back byte for byte");

    let (status, _, _) = call!(app, photo(&jwt, "DELETE").to_request());
    assert_eq!(status, StatusCode::NO_CONTENT);
    let me = json_of(&call!(app, get_me(&jwt).to_request()).2);
    assert_eq!(me["has_photo"], json!(false));
    assert_eq!(
        call!(app, photo(&jwt, "GET").to_request()).0,
        StatusCode::NOT_FOUND
    );
    // Removing it again is not an error.
    assert_eq!(
        call!(app, photo(&jwt, "DELETE").to_request()).0,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
#[serial(process_env)]
async fn mair_1231_photo_refuses_other_formats_and_large_images() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (_, jwt) = signed_in_agent(&state, &raw).await;

    let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01".to_vec();
    let (status, _, body) = call!(app, put_photo(&jwt, png).to_request());
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8(body).unwrap().contains("photo"));
    assert_eq!(
        call!(app, put_photo(&jwt, Vec::new()).to_request()).0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call!(app, put_photo(&jwt, webp(100 * 1024 + 1)).to_request()).0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let me = json_of(&call!(app, get_me(&jwt).to_request()).2);
    assert_eq!(me["has_photo"], json!(false));
}

#[tokio::test]
#[serial(process_env)]
async fn mair_1231_photo_routes_need_a_session() {
    let (state, _raw, _redis) = setup().await;
    let app = init_app!(state);
    let anonymous = test::TestRequest::put()
        .uri("/api/v1/user/me/photo")
        .set_payload(webp(64))
        .to_request();
    assert_eq!(call!(app, anonymous).0, StatusCode::UNAUTHORIZED);
    let anonymous = test::TestRequest::get()
        .uri("/api/v1/user/me/photo")
        .to_request();
    assert_eq!(call!(app, anonymous).0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial(process_env)]
async fn mair_1231_get_me_returns_the_service_of_the_agent() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user_id, jwt) = signed_in_agent(&state, &raw).await;
    let name = format!("Voirie {}", &Uuid::new_v4().to_string()[..8]);
    let service: i32 = sqlx::query("INSERT INTO services (name) VALUES ($1) RETURNING id")
        .bind(&name)
        .fetch_one(&raw)
        .await
        .unwrap()
        .get("id");
    sqlx::query("UPDATE users SET service_id = $1 WHERE id = $2")
        .bind(service)
        .bind(i32::try_from(user_id).unwrap())
        .execute(&raw)
        .await
        .unwrap();

    let me = json_of(&call!(app, get_me(&jwt).to_request()).2);
    assert_eq!(me["service"], json!({ "id": service, "name": name }));
}

#[tokio::test]
#[serial(process_env)]
async fn mair_1231_anonymizing_the_agent_clears_its_profile_details() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user_id, jwt) = signed_in_agent(&state, &raw).await;
    let body = json!({ "biography": "Bio", "job_title": "Chargée de mission" });
    assert_eq!(
        call!(app, patch_me(&jwt, &body).to_request()).0,
        StatusCode::OK
    );
    assert_eq!(
        call!(app, put_photo(&jwt, webp(256)).to_request()).0,
        StatusCode::NO_CONTENT
    );

    sqlx::query("SELECT anonymize_user($1)")
        .bind(i32::try_from(user_id).unwrap())
        .execute(&raw)
        .await
        .unwrap();
    let row = sqlx::query(
        "SELECT biography IS NULL AND job_title IS NULL AND photo IS NULL AS cleared FROM users WHERE id = $1",
    )
    .bind(i32::try_from(user_id).unwrap())
    .fetch_one(&raw)
    .await
    .unwrap();
    assert!(row.get::<bool, _>("cleared"));
}
