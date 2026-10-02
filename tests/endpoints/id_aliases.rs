//! An id beyond the `INT4` range no longer wraps onto another row (MAIR-422):
//! `2^32 + id` used to read the row `id`.

use crate::common::{get_pool, get_raw_pool, users::create_user, users::unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ADMIN_ID};
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serial_test::serial;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as `sessions_refresh`: both modules run in the same test binary.
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

/// `2^32`: added to an id, an `as i32` cast gives the id back.
const WRAP: u64 = 1 << 32;

#[tokio::test]
#[serial]
async fn ids_beyond_int4_do_not_alias_another_row() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .configure(public_config)
            .service(
                web::scope("/api")
                    .wrap(actix_web::middleware::from_fn(session_guard))
                    .wrap(JwtMiddleware)
                    .configure(config),
            ),
    )
    .await;
    let user = create_user(&pool, "Alias", &unique_marker("alias")).await;
    let group: i32 = sqlx::query_scalar(
        "INSERT INTO groups (owner_id, name, description) VALUES ($1, $2, 'alias') RETURNING id",
    )
    .bind(user)
    .bind(format!("alias-{}", uuid::Uuid::new_v4()))
    .fetch_one(&raw)
    .await
    .unwrap();
    let admin = generate_jwt(&ADMIN_ID.get().unwrap().to_string(), "admin").unwrap();
    let user_id = u64::try_from(user).unwrap();
    let group_id = u64::try_from(group).unwrap();

    for (uri, real) in [
        (
            format!("/api/v1/user/{}/", user_id + WRAP),
            format!("/api/v1/user/{user_id}/"),
        ),
        (
            format!("/api/v1/admin/users/{}/", user_id + WRAP),
            format!("/api/v1/admin/users/{user_id}/"),
        ),
        (
            format!("/api/v1/groups/{}/", group_id + WRAP),
            format!("/api/v1/groups/{group_id}/"),
        ),
    ] {
        let get = |uri: &str| {
            test::TestRequest::get()
                .uri(uri)
                .insert_header(("Authorization", format!("Bearer {admin}")))
                .to_request()
        };
        let status = match test::try_call_service(&app, get(&real)).await {
            Ok(resp) => resp.status(),
            Err(err) => err.as_response_error().status_code(),
        };
        assert_eq!(status, StatusCode::OK, "{real}");
        let status = match test::try_call_service(&app, get(&uri)).await {
            Ok(resp) => resp.status(),
            Err(err) => err.as_response_error().status_code(),
        };
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }
}
