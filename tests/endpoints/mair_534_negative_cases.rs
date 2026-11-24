//! MAIR-534: business negative cases of the role administration, the admin password reset and the session
//! revocation, which only had their access sweep (401 / 403).

use crate::common::{
    core_db_url, get_pool, get_raw_pool, users::create_user, users::unique_marker,
};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::refresh_token;
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::ADMIN_ID;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::json;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    mairie360_api_lib::test_setup::http_setup::init_test_jwt_env();
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

macro_rules! call {
    ($app:expr, $request:expr) => {
        test::call_service(&$app, $request.to_request())
            .await
            .status()
    };
}

#[tokio::test]
async fn role_creation_and_update_refuse_bad_input_and_duplicates() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let app = init_app!(state);
    let name = format!("Role {}", unique_marker("role"));
    let other = format!("Other {}", unique_marker("role"));
    let post = |body: serde_json::Value| {
        test::TestRequest::post()
            .uri("/api/v1/admin/roles/")
            .insert_header(admin_bearer())
            .set_json(body)
    };

    assert_eq!(
        call!(app, post(json!({ "name": name, "description": "Agent" }))),
        StatusCode::OK
    );
    assert_eq!(
        call!(app, post(json!({ "name": other, "description": "Autre" }))),
        StatusCode::OK
    );
    assert_eq!(
        call!(app, post(json!({ "name": name, "description": "Doublon" }))),
        StatusCode::CONFLICT
    );
    for body in [
        json!({ "name": "  ", "description": "Vide" }),
        json!({ "description": "Sans nom" }),
        json!({ "name": "x".repeat(65), "description": "Trop long" }),
    ] {
        assert_eq!(call!(app, post(body)), StatusCode::BAD_REQUEST);
    }

    let raw = get_raw_pool(crate::common::own_db_url().await).await;
    let id: i32 = sqlx::query_scalar("SELECT id FROM roles WHERE name = $1")
        .bind(&name)
        .fetch_one(&raw)
        .await
        .unwrap();
    let patch = |body: serde_json::Value| {
        test::TestRequest::patch()
            .uri(&format!("/api/v1/admin/roles/{id}"))
            .insert_header(admin_bearer())
            .set_json(body)
    };
    let put = |body: serde_json::Value| {
        test::TestRequest::put()
            .uri(&format!("/api/v1/admin/roles/{id}"))
            .insert_header(admin_bearer())
            .set_json(body)
    };

    assert_eq!(
        call!(app, patch(json!({ "description": "Agent de mairie" }))),
        StatusCode::OK
    );
    assert_eq!(
        call!(
            app,
            put(json!({ "name": name, "description": "Remplacé", "can_be_deleted": true }))
        ),
        StatusCode::OK
    );
    assert_eq!(
        call!(app, patch(json!({ "name": other }))),
        StatusCode::CONFLICT
    );
    assert_eq!(
        call!(app, patch(json!({ "name": "" }))),
        StatusCode::BAD_REQUEST
    );
    let unknown = test::TestRequest::patch()
        .uri("/api/v1/admin/roles/2147483000")
        .insert_header(admin_bearer())
        .set_json(json!({ "description": "x" }));
    assert_eq!(call!(app, unknown), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_system_role_cannot_be_renamed() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let app = init_app!(state);
    let raw = get_raw_pool(crate::common::own_db_url().await).await;
    let admin_role: i32 = sqlx::query_scalar("SELECT id FROM roles WHERE name = 'Admin'")
        .fetch_one(&raw)
        .await
        .unwrap();

    let rename = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/roles/{admin_role}"))
        .insert_header(admin_bearer())
        .set_json(json!({ "name": "Administrateur" }));

    assert_eq!(call!(app, rename), StatusCode::FORBIDDEN);
    let still: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles WHERE name = 'Admin'")
        .fetch_one(&raw)
        .await
        .unwrap();
    assert_eq!(still, 1, "the system role keeps its name");
}

#[tokio::test]
async fn admin_password_reset_refuses_a_weak_password_and_an_unknown_user() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let pool = get_pool().await;
    let app = init_app!(state);
    let user = create_user(&pool, "Reset", &unique_marker("reset")).await;
    let reset = |id: i64, password: &str| {
        test::TestRequest::patch()
            .uri(&format!("/api/v1/admin/users/{id}/password"))
            .insert_header(admin_bearer())
            .set_json(json!({ "new_password": password }))
    };

    assert_eq!(
        call!(app, reset(i64::from(user), "short")),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call!(app, reset(2_147_483_000, "UnBonMotDePasse!123")),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call!(app, reset(i64::from(user), "UnBonMotDePasse!123")),
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn a_user_cannot_revoke_the_session_of_another_user() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let pool = get_pool().await;
    let app = init_app!(state);
    let owner = create_user(&pool, "Owner", &unique_marker("rev")).await;
    let intruder = create_user(&pool, "Intruder", &unique_marker("rev")).await;
    let token = format!("revoke_other_{}", uuid::Uuid::new_v4());
    state
        .get_smart_db()
        .execute(
            core_api::database::sessions::create_session::CreateSessionQueryView::new(
                u64::try_from(owner).unwrap(),
                &refresh_token::hash(&token),
                "owner_device",
                std::net::IpAddr::from([0, 0, 0, 0]),
            ),
        )
        .await
        .unwrap();
    let intruder_jwt = generate_jwt(&intruder.to_string(), "user").unwrap();
    let revoke = |jwt: &str| {
        test::TestRequest::post()
            .uri("/api/v1/sessions/revoke")
            .insert_header(("Authorization", format!("Bearer {jwt}")))
            .set_json(json!({ "refresh_token": token }))
    };

    assert_eq!(call!(app, revoke(&intruder_jwt)), StatusCode::UNAUTHORIZED);
    // The owner's session is untouched: its owner can still revoke it.
    let owner_jwt = generate_jwt(&owner.to_string(), "user").unwrap();
    assert_eq!(call!(app, revoke(&owner_jwt)), StatusCode::OK);
}
