//! A user holds one role (MAIR-564): granting a role replaces the former one, and the responses
//! expose it as `role`.

use crate::common::{
    core_db_url, get_pool, get_raw_pool, users::create_user, users::unique_marker,
};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ADMIN_ID};
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::PgPool;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as `sessions_refresh`: both modules run in the same test binary.
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

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

fn admin_bearer() -> (&'static str, String) {
    let token = generate_jwt(&ADMIN_ID.get().unwrap().to_string(), "admin").unwrap();
    ("Authorization", format!("Bearer {token}"))
}

async fn role_id(raw: &PgPool, name: &str) -> i32 {
    sqlx::query_scalar("SELECT id FROM roles WHERE name = $1")
        .bind(name)
        .fetch_one(raw)
        .await
        .unwrap()
}

/// Names of the roles `user_id` holds in the database.
async fn role_names(raw: &PgPool, user_id: i32) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT r.name FROM user_roles ur JOIN roles r ON r.id = ur.role_id \
         WHERE ur.user_id = $1 ORDER BY r.id",
    )
    .bind(user_id)
    .fetch_all(raw)
    .await
    .unwrap()
}

macro_rules! grant {
    ($app:expr, $user:expr, $role:expr) => {
        test::call_service(
            &$app,
            test::TestRequest::post()
                .uri(&format!("/api/v1/admin/users/{}/roles/", $user))
                .insert_header(admin_bearer())
                .set_json(json!({ "role_id": $role }))
                .to_request(),
        )
        .await
        .status()
    };
}

macro_rules! get_json {
    ($app:expr, $uri:expr) => {{
        let resp = test::call_service(
            &$app,
            test::TestRequest::get()
                .uri(&$uri)
                .insert_header(admin_bearer())
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK, "GET {}", $uri);
        let body: Value = test::read_body_json(resp).await;
        body
    }};
}

#[tokio::test]
#[serial]
async fn mair_564_granting_a_role_replaces_the_former_one() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let pool = get_pool().await;
    let raw = get_raw_pool(host.clone()).await;
    let app = init_app!(state);
    // Registered without a role: the schema gives it Guest.
    let user = create_user(&pool, "Single", &unique_marker("onerole")).await;
    assert_eq!(role_names(&raw, user).await, vec!["Guest"]);

    assert_eq!(
        grant!(app, user, role_id(&raw, "Responsable").await),
        StatusCode::OK
    );
    assert_eq!(role_names(&raw, user).await, vec!["Responsable"]);

    // A second grant replaces the role instead of adding one.
    let user_role = role_id(&raw, "User").await;
    assert_eq!(grant!(app, user, user_role), StatusCode::OK);
    assert_eq!(role_names(&raw, user).await, vec!["User"]);

    // The role already held: nothing changes.
    assert_eq!(grant!(app, user, user_role), StatusCode::CONFLICT);
    assert_eq!(role_names(&raw, user).await, vec!["User"]);
}

#[tokio::test]
#[serial]
async fn mair_564_unknown_role_keeps_the_former_one() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let pool = get_pool().await;
    let raw = get_raw_pool(host.clone()).await;
    let app = init_app!(state);
    let user = create_user(&pool, "Kept", &unique_marker("onerole")).await;
    assert_eq!(
        grant!(app, user, role_id(&raw, "Maire").await),
        StatusCode::OK
    );

    assert_eq!(grant!(app, user, 999_999), StatusCode::NOT_FOUND);
    assert_eq!(role_names(&raw, user).await, vec!["Maire"]);
}

#[tokio::test]
#[serial]
async fn mair_564_responses_expose_one_role() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let pool = get_pool().await;
    let raw = get_raw_pool(host.clone()).await;
    let app = init_app!(state);
    let marker = unique_marker("onerole");
    let user = create_user(&pool, "Shown", &marker).await;
    let responsable = role_id(&raw, "Responsable").await;
    assert_eq!(grant!(app, user, responsable), StatusCode::OK);

    let record = get_json!(app, format!("/api/v1/user/{user}/"));
    assert_eq!(record["role"], json!("Responsable"));
    assert_eq!(record["roles"], json!(["Responsable"]));

    let directory = get_json!(app, format!("/api/v1/user/?ids={user}"));
    assert_eq!(directory["users"][0]["role"], json!("Responsable"));
    assert_eq!(directory["users"][0]["roles"], json!(["Responsable"]));

    let admin_record = get_json!(app, format!("/api/v1/admin/users/{user}/"));
    assert_eq!(admin_record["role"]["id"], json!(responsable));
    assert_eq!(admin_record["role"]["name"], json!("Responsable"));
    assert_eq!(admin_record["roles"].as_array().unwrap().len(), 1);

    let admin_list = get_json!(app, format!("/api/v1/admin/users/?search={marker}"));
    let row = &admin_list["users"][0];
    assert_eq!(row["id"], json!(user));
    assert_eq!(
        row["role"],
        json!({ "id": responsable, "name": "Responsable" })
    );
    assert_eq!(row["roles"].as_array().unwrap().len(), 1);

    // Without any role: empty string or `null`, never a missing field.
    sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
        .bind(user)
        .execute(&raw)
        .await
        .unwrap();
    let record = get_json!(app, format!("/api/v1/user/{user}/"));
    assert_eq!(record["role"], json!(""));
    let admin_list = get_json!(app, format!("/api/v1/admin/users/?search={marker}"));
    assert_eq!(admin_list["users"][0]["role"], Value::Null);
}
