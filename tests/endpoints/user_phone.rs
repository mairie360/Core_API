//! Phone numbers (MAIR-480): a country and a number, validated against the numbering plan of
//! the country, cleared with `null` or `""`, read back in E.164.

use crate::common::{get_pool, users::create_user, users::unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use serial_test::serial;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as `sessions_refresh`: both modules run in the same test binary.
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

const ME: &str = "/api/v1/user/me/";
const ADMIN_USERS: &str = "/api/v1/admin/users/";

/// Same mounting as `main.rs`.
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

macro_rules! send {
    ($app:expr, $req:expr) => {{
        let resp = test::call_service(&$app, $req.to_request()).await;
        let status = resp.status();
        let body = test::read_body(resp).await;
        (status, String::from_utf8_lossy(&body).to_string())
    }};
}

fn bearer(user_id: i32, role: &str) -> (&'static str, String) {
    (
        "Authorization",
        format!(
            "Bearer {}",
            generate_jwt(&user_id.to_string(), role).unwrap()
        ),
    )
}

#[tokio::test]
#[serial]
async fn profile_phone_is_set_read_in_e164_and_cleared() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let pool = get_pool(host.clone()).await;
    let app = init_app!(state);
    let user = create_user(&pool, "Phone", &unique_marker("phone")).await;
    let auth = bearer(user, "user");

    let patch = |body: Value| {
        test::TestRequest::patch()
            .uri(ME)
            .insert_header(auth.clone())
            .set_json(body)
    };
    let read = || async {
        let (status, body) = send!(
            app,
            test::TestRequest::get().uri(ME).insert_header(auth.clone())
        );
        assert_eq!(status, StatusCode::OK, "{body}");
        let body: Value = serde_json::from_str(&body).unwrap();
        (body["phone"].clone(), body["phone_country"].clone())
    };

    // National format with its country.
    let (status, body) = send!(
        app,
        patch(json!({ "phone": "06 12 34 56 78", "phone_country": "FR" }))
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(read().await, (json!("+33612345678"), json!("FR")));

    // International number in E.164, no country needed.
    let (status, body) = send!(app, patch(json!({ "phone": "+32 470 12 34 56" })));
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(read().await, (json!("+32470123456"), json!("BE")));

    // Another field alone keeps the phone.
    let (status, body) = send!(app, patch(json!({ "first_name": "Renamed" })));
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(read().await, (json!("+32470123456"), json!("BE")));

    // `null` clears it.
    let (status, body) = send!(app, patch(json!({ "phone": null })));
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(read().await, (Value::Null, Value::Null));

    // `""` clears it too.
    let (status, body) = send!(
        app,
        patch(json!({ "phone": "0798765432", "phone_country": "FR" }))
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(read().await, (json!("+33798765432"), json!("FR")));
    let (status, body) = send!(app, patch(json!({ "phone": "" })));
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(read().await, (Value::Null, Value::Null));
}

#[tokio::test]
#[serial]
async fn invalid_profile_phones_answer_400_and_change_nothing() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let pool = get_pool(host.clone()).await;
    let app = init_app!(state);
    let user = create_user(&pool, "Phone", &unique_marker("phone_400")).await;
    let auth = bearer(user, "user");
    let (status, body) = send!(
        app,
        test::TestRequest::patch()
            .uri(ME)
            .insert_header(auth.clone())
            .set_json(json!({ "phone": "0612345678", "phone_country": "FR" }))
    );
    assert_eq!(status, StatusCode::OK, "{body}");

    for (body, field) in [
        (json!({ "phone": "0612", "phone_country": "FR" }), "phone"),
        (json!({ "phone": "0612345678" }), "phone"),
        (
            json!({ "phone": "0612345678", "phone_country": "France" }),
            "phone_country",
        ),
        (
            json!({ "phone": "0612345678", "phone_country": "fr" }),
            "phone_country",
        ),
        (json!({ "phone_country": "BE" }), "phone_country"),
        (
            json!({ "phone": "06 12 34 56 7a", "phone_country": "FR" }),
            "phone",
        ),
        (
            json!({ "phone": "../../etc/passwd", "phone_country": "FR" }),
            "phone",
        ),
        (
            json!({ "phone": "0".repeat(33), "phone_country": "FR" }),
            "phone",
        ),
    ] {
        let (status, answer) = send!(
            app,
            test::TestRequest::patch()
                .uri(ME)
                .insert_header(auth.clone())
                .set_json(&body)
        );
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {answer}");
        assert!(
            answer.starts_with(&format!("Invalid `{field}`")),
            "{body}: {answer}"
        );
    }

    let (_, body) = send!(app, test::TestRequest::get().uri(ME).insert_header(auth));
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["phone"], "+33612345678");
}

#[tokio::test]
#[serial]
async fn admin_creates_and_clears_a_phone() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let app = init_app!(state);
    let admin = bearer(1, "admin");
    let email = format!("{}@example.com", unique_marker("admin_phone"));

    let create = |body: Value| {
        test::TestRequest::post()
            .uri(ADMIN_USERS)
            .insert_header(admin.clone())
            .set_json(body)
    };
    let base = json!({
        "first_name": "Claire",
        "last_name": "Martin",
        "email": email,
        "password": "MotDePasse!123",
    });
    let with = |extra: Value| {
        let mut body = base.clone();
        body.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        body
    };

    let (status, body) = send!(app, create(with(json!({ "phone_number": "0612345678" }))));
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let (status, body) = send!(
        app,
        create(with(
            json!({ "phone_number": "+262 692 12 34 56", "phone_country": "FR" })
        ))
    );
    assert_eq!(status, StatusCode::CREATED, "{body}");

    let (status, body) = send!(
        app,
        test::TestRequest::get()
            .uri(&format!("{ADMIN_USERS}?search={email}"))
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    let list: Value = serde_json::from_str(&body).unwrap();
    let row = &list["users"][0];
    // The number's own country is stored, not the one picked.
    assert_eq!(row["phone_number"], "+262692123456");
    assert_eq!(row["phone_country"], "RE");
    let id = row["id"].as_i64().unwrap();

    let (status, body) = send!(
        app,
        test::TestRequest::patch()
            .uri(&format!("{ADMIN_USERS}{id}/"))
            .insert_header(admin.clone())
            .set_json(json!({ "phone_number": null }))
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = send!(
        app,
        test::TestRequest::get()
            .uri(&format!("{ADMIN_USERS}{id}/"))
            .insert_header(admin)
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    let user: Value = serde_json::from_str(&body).unwrap();
    assert!(user["user"]["phone_number"].is_null(), "{user}");
    assert!(user["user"]["phone_country"].is_null(), "{user}");
}
