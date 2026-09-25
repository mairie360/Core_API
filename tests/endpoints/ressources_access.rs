//! MAIR-221: only an administrator or the owner of a resource instance may add, remove or list
//! its accesses.

use crate::common::{get_pool, users::create_user, users::unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ADMIN_ID, GROUP_OWNER_ID};
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use serial_test::serial;

/// Seeded group owned by `GROUP_OWNER_ID` (see `mairie360_api_lib::test_setup`).
const OWNED_GROUP_ID: u64 = 1;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as `sessions_refresh`: both modules run in the same test binary.
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

/// Same wiring as `main.rs`: public routes, then the `/api` scope behind `JwtMiddleware`.
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

macro_rules! call {
    ($app:expr, $req:expr) => {
        match test::try_call_service(&$app, $req).await {
            Ok(resp) => {
                let status = resp.status();
                let body = test::read_body(resp).await;
                (status, String::from_utf8_lossy(&body).to_string())
            }
            Err(err) => (err.as_response_error().status_code(), err.to_string()),
        }
    };
}

fn bearer(user_id: i32) -> (&'static str, String) {
    let token = generate_jwt(&user_id.to_string(), "user").unwrap();
    ("Authorization", format!("Bearer {token}"))
}

fn admin() -> i32 {
    *ADMIN_ID.get().unwrap()
}

fn owner() -> i32 {
    *GROUP_OWNER_ID.get().unwrap()
}

/// App state and two fresh accounts without any role but the default one: the target of the
/// accesses, and an unrelated caller.
async fn setup() -> (web::Data<AppState>, i32, i32) {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let pool = get_pool(host.clone()).await;
    let target = create_user(&pool, "Target", &unique_marker("acl")).await;
    let other = create_user(&pool, "Other", &unique_marker("acl")).await;
    (state, target, other)
}

fn add_request(caller: i32, target: i32, ressource_type: &str, instance: u64) -> test::TestRequest {
    test::TestRequest::post()
        .uri("/api/v1/ressources/add_access")
        .insert_header(bearer(caller))
        .set_json(json!({
            "user_id": target,
            "resource_id": instance,
            "ressource_type": ressource_type,
            "access_type": "Read"
        }))
}

fn list_request(caller: i32, ressource_type: &str, instance: u64) -> test::TestRequest {
    test::TestRequest::post()
        .uri(&format!(
            "/api/v1/ressources/{instance}/access?ressource_type={ressource_type}"
        ))
        .insert_header(bearer(caller))
}

fn remove_request(caller: i32, access_id: i64) -> test::TestRequest {
    test::TestRequest::post()
        .uri("/api/v1/ressources/remove_access")
        .insert_header(bearer(caller))
        .set_json(json!({ "access_id": access_id }))
}

/// Id of the entry granted to `target` in a `list` response body.
fn entry_of(body: &str, target: i32) -> Option<i64> {
    let value: Value = serde_json::from_str(body).unwrap();
    value["accesses"]
        .as_array()
        .unwrap()
        .iter()
        .find(|access| access["user_id"] == json!(target))
        .and_then(|access| access["id"].as_i64())
}

#[tokio::test]
#[serial]
async fn admin_can_add_list_and_remove_accesses() {
    let (state, target, _) = setup().await;
    let app = init_app!(state);

    let (status, body) = call!(
        app,
        add_request(admin(), target, "groups", OWNED_GROUP_ID).to_request()
    );
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = call!(
        app,
        list_request(admin(), "groups", OWNED_GROUP_ID).to_request()
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    let access_id = entry_of(&body, target).expect("granted entry should be listed");

    let (status, body) = call!(app, remove_request(admin(), access_id).to_request());
    assert_eq!(status, StatusCode::OK, "{body}");

    let (_, body) = call!(
        app,
        list_request(admin(), "groups", OWNED_GROUP_ID).to_request()
    );
    assert_eq!(entry_of(&body, target), None, "{body}");
}

#[tokio::test]
#[serial]
async fn admin_can_manage_a_type_without_owner() {
    let (state, target, _) = setup().await;
    let app = init_app!(state);

    let (status, body) = call!(
        app,
        add_request(admin(), target, "users", u64::try_from(target).unwrap()).to_request()
    );
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
#[serial]
async fn owner_can_add_list_and_remove_accesses() {
    let (state, target, _) = setup().await;
    let app = init_app!(state);

    let (status, body) = call!(
        app,
        add_request(owner(), target, "groups", OWNED_GROUP_ID).to_request()
    );
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = call!(
        app,
        list_request(owner(), "groups", OWNED_GROUP_ID).to_request()
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    let access_id = entry_of(&body, target).expect("granted entry should be listed");

    let (status, body) = call!(app, remove_request(owner(), access_id).to_request());
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
#[serial]
async fn other_user_gets_403_everywhere() {
    let (state, target, other) = setup().await;
    let app = init_app!(state);

    // Granting themselves an access on a group they do not own.
    let (status, body) = call!(
        app,
        add_request(other, other, "groups", OWNED_GROUP_ID).to_request()
    );
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    // Types without owner are admin-only, even on their own account.
    let (status, body) = call!(
        app,
        add_request(other, other, "users", u64::try_from(other).unwrap()).to_request()
    );
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    let (status, body) = call!(
        app,
        list_request(other, "groups", OWNED_GROUP_ID).to_request()
    );
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    // An existing entry of someone else's group cannot be removed.
    let (status, _) = call!(
        app,
        add_request(admin(), target, "groups", OWNED_GROUP_ID).to_request()
    );
    assert_eq!(status, StatusCode::OK);
    let (_, body) = call!(
        app,
        list_request(admin(), "groups", OWNED_GROUP_ID).to_request()
    );
    let access_id = entry_of(&body, target).unwrap();
    let (status, body) = call!(app, remove_request(other, access_id).to_request());
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    // A missing entry looks the same as someone else's entry.
    let (status, body) = call!(app, remove_request(other, i64::from(i32::MAX)).to_request());
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    // The entry is still there.
    let (_, body) = call!(
        app,
        list_request(admin(), "groups", OWNED_GROUP_ID).to_request()
    );
    assert_eq!(entry_of(&body, target), Some(access_id), "{body}");
    let (status, _) = call!(app, remove_request(admin(), access_id).to_request());
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn unauthenticated_calls_get_401() {
    let (state, _, _) = setup().await;
    let app = init_app!(state);

    let requests = [
        test::TestRequest::post()
            .uri("/api/v1/ressources/add_access")
            .set_json(json!({
                "user_id": 1,
                "resource_id": OWNED_GROUP_ID,
                "ressource_type": "groups",
                "access_type": "Read"
            })),
        test::TestRequest::post().uri("/api/v1/ressources/1/access?ressource_type=groups"),
        test::TestRequest::post()
            .uri("/api/v1/ressources/remove_access")
            .set_json(json!({ "access_id": 1 })),
    ];
    for req in requests {
        let (status, body) = call!(app, req.to_request());
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    }
}

#[tokio::test]
#[serial]
async fn client_errors_are_not_500() {
    let (state, target, _) = setup().await;
    let app = init_app!(state);

    let (status, body) = call!(
        app,
        add_request(admin(), target, "not_a_resource", 1).to_request()
    );
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let (status, body) = call!(
        app,
        add_request(admin(), i32::MAX, "groups", OWNED_GROUP_ID).to_request()
    );
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let (status, body) = call!(app, list_request(admin(), "not_a_resource", 1).to_request());
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let (status, body) = call!(
        app,
        remove_request(admin(), i64::from(i32::MAX)).to_request()
    );
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");

    // Values Postgres would reject (out of `INT` range, NUL byte) are client errors too.
    let (status, body) = call!(
        app,
        add_request(admin(), target, "groups", u64::from(u32::MAX)).to_request()
    );
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let (status, body) = call!(
        app,
        add_request(admin(), target, "gro\u{0}ups", OWNED_GROUP_ID).to_request()
    );
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let (status, _) = call!(
        app,
        add_request(admin(), target, "groups", OWNED_GROUP_ID).to_request()
    );
    assert_eq!(status, StatusCode::OK);
    let (status, body) = call!(
        app,
        add_request(admin(), target, "groups", OWNED_GROUP_ID).to_request()
    );
    assert_eq!(
        status,
        StatusCode::OK,
        "granting twice is idempotent: {body}"
    );

    let (_, body) = call!(
        app,
        list_request(admin(), "groups", OWNED_GROUP_ID).to_request()
    );
    let value: Value = serde_json::from_str(&body).unwrap();
    let granted = value["accesses"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|access| access["user_id"] == json!(target))
        .count();
    assert_eq!(granted, 1, "{body}");
    let access_id = entry_of(&body, target).unwrap();
    let (status, _) = call!(app, remove_request(admin(), access_id).to_request());
    assert_eq!(status, StatusCode::OK);
}
