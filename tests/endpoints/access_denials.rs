//! Access refusals (MAIR-419): the authorization logic lives in `src/endpoints/`, so every
//! refusal path is pinned here by an integration test rather than left out of the coverage gate.
//!
//! The admin and authentication checks are driven by the published contract (`ApiDoc`), so a new
//! operation is covered as soon as it is documented.

use crate::common::{get_pool, get_raw_pool, users::create_user, users::unique_marker};
use actix_web::{http::Method, http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::swagger::ApiDoc;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ADMIN_ID};
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::{PgPool, Row};
use utoipa::OpenApi;

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

/// Status of a call, whether a handler answered or a middleware refused it.
macro_rules! status {
    ($app:expr, $req:expr) => {
        match test::try_call_service(&$app, $req).await {
            Ok(resp) => resp.status(),
            Err(err) => err.as_response_error().status_code(),
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

async fn setup() -> (web::Data<AppState>, PgPool, i32, i32) {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let pool = get_pool(host.clone()).await;
    let caller = create_user(&pool, "Caller", &unique_marker("deny")).await;
    let target = create_user(&pool, "Target", &unique_marker("deny")).await;
    (state, get_raw_pool(host.clone()).await, caller, target)
}

/// Every published `/api/v1` operation as `(method, uri, secured)`, path parameters replaced by
/// `id`. `secured` is whether the operation declares the `jwt` security scheme.
fn published_operations(id: i32) -> Vec<(Method, String, bool)> {
    let document = serde_json::to_value(ApiDoc::openapi()).unwrap();
    let mut operations = Vec::new();
    for (template, methods) in document["paths"].as_object().unwrap() {
        if !template.starts_with("/api/v1/") {
            continue;
        }
        let uri = template
            .split('/')
            .map(|segment| {
                if segment.starts_with('{') {
                    id.to_string()
                } else {
                    segment.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("/");
        for (method, operation) in methods.as_object().unwrap() {
            let secured = operation["security"]
                .as_array()
                .is_some_and(|schemes| schemes.iter().any(|scheme| scheme.get("jwt").is_some()));
            operations.push((
                Method::from_bytes(method.to_uppercase().as_bytes()).unwrap(),
                uri.clone(),
                secured,
            ));
        }
    }
    operations
}

async fn new_group(raw: &PgPool, owner: i32) -> i32 {
    sqlx::query(
        "INSERT INTO groups (owner_id, name, description) VALUES ($1, $2, 'test') RETURNING id",
    )
    .bind(owner)
    .bind(format!("deny-{}", uuid::Uuid::new_v4()))
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id")
}

async fn add_member(raw: &PgPool, group_id: i32, user_id: i32) {
    sqlx::query(
        "INSERT INTO group_members (group_id, user_id) VALUES ($1, $2) \
         ON CONFLICT DO NOTHING",
    )
    .bind(group_id)
    .bind(user_id)
    .execute(raw)
    .await
    .unwrap();
}

#[tokio::test]
#[serial]
async fn every_admin_operation_refuses_a_non_admin() {
    let (state, raw, caller, target) = setup().await;
    let app = init_app!(state);

    let admin_operations: Vec<_> = published_operations(target)
        .into_iter()
        .filter(|(_, uri, _)| uri.starts_with("/api/v1/admin/"))
        .collect();
    assert!(admin_operations.len() >= 10, "{admin_operations:?}");

    for (method, uri, _) in admin_operations {
        // The encoded spelling reaches the same handlers through the actix router.
        let encoded = uri.replacen("/admin/", "/%61dmin/", 1);
        for uri in [&uri, &encoded] {
            let req = test::TestRequest::default()
                .method(method.clone())
                .uri(uri)
                .insert_header(bearer(caller))
                .set_json(json!({}))
                .to_request();
            assert_eq!(status!(app, req), StatusCode::FORBIDDEN, "{method} {uri}");
        }
    }

    // None of the refused writes went through: the target is still active, with its name.
    let row = sqlx::query("SELECT is_archived, first_name FROM users WHERE id = $1")
        .bind(target)
        .fetch_one(&raw)
        .await
        .unwrap();
    assert!(!row.get::<bool, _>("is_archived"));
    assert_eq!(row.get::<String, _>("first_name"), "Target");
}

#[tokio::test]
#[serial]
async fn every_secured_operation_refuses_a_missing_or_forged_token() {
    let (state, _, _, target) = setup().await;
    let app = init_app!(state);

    let secured: Vec<_> = published_operations(target)
        .into_iter()
        .filter(|(_, _, secured)| *secured)
        .collect();
    assert_ne!(secured.len(), 0);

    for (method, uri, _) in secured {
        let req = test::TestRequest::default()
            .method(method.clone())
            .uri(&uri)
            .to_request();
        assert_eq!(
            status!(app, req),
            StatusCode::UNAUTHORIZED,
            "{method} {uri}"
        );

        let req = test::TestRequest::default()
            .method(method.clone())
            .uri(&uri)
            .insert_header(("Authorization", "Bearer not.a.jwt"))
            .to_request();
        assert_eq!(
            status!(app, req),
            StatusCode::UNAUTHORIZED,
            "{method} {uri}"
        );
    }
}

#[tokio::test]
#[serial]
async fn another_user_cannot_modify_or_delete_a_group() {
    let (state, raw, caller, owner) = setup().await;
    let app = init_app!(state);
    let group = new_group(&raw, owner).await;
    add_member(&raw, group, owner).await;

    let requests = [
        test::TestRequest::patch()
            .uri(&format!("/api/v1/groups/{group}/"))
            .set_json(json!({ "name": format!("taken-{}", uuid::Uuid::new_v4()) })),
        test::TestRequest::delete().uri(&format!("/api/v1/groups/{group}/")),
        test::TestRequest::post()
            .uri(&format!("/api/v1/groups/{group}/users/"))
            .set_json(json!({ "user_id": caller })),
        test::TestRequest::delete().uri(&format!("/api/v1/groups/{group}/users/{owner}/")),
        test::TestRequest::get().uri(&format!("/api/v1/groups/{group}/")),
        test::TestRequest::get().uri(&format!("/api/v1/groups/{group}/users/")),
    ];
    for req in requests {
        let req = req.insert_header(bearer(caller)).to_request();
        let label = format!("{} {}", req.method(), req.uri());
        assert_eq!(status!(app, req), StatusCode::FORBIDDEN, "{label}");
    }

    let row = sqlx::query(
        "SELECT g.name LIKE 'deny-%' AS unchanged, \
         EXISTS (SELECT 1 FROM group_members m WHERE m.group_id = g.id AND m.user_id = $2) \
           AS owner_member, \
         EXISTS (SELECT 1 FROM group_members m WHERE m.group_id = g.id AND m.user_id = $3) \
           AS caller_member \
         FROM groups g WHERE g.id = $1",
    )
    .bind(group)
    .bind(owner)
    .bind(caller)
    .fetch_one(&raw)
    .await
    .expect("the group still exists");
    assert!(row.get::<bool, _>("unchanged"));
    assert!(row.get::<bool, _>("owner_member"));
    assert!(!row.get::<bool, _>("caller_member"));
}

#[tokio::test]
#[serial]
async fn the_owner_can_modify_and_delete_its_group() {
    let (state, raw, member, owner) = setup().await;
    let app = init_app!(state);
    let group = new_group(&raw, owner).await;
    add_member(&raw, group, member).await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/groups/{group}/"))
        .insert_header(bearer(owner))
        .set_json(json!({ "description": "Updated by its owner" }))
        .to_request();
    assert_eq!(status!(app, req), StatusCode::OK);
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/groups/{group}/users/{member}/"))
        .insert_header(bearer(owner))
        .to_request();
    assert!(status!(app, req).is_success());
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/groups/{group}/"))
        .insert_header(bearer(owner))
        .to_request();
    assert_eq!(status!(app, req), StatusCode::NO_CONTENT);

    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM groups WHERE id = $1)")
        .bind(group)
        .fetch_one(&raw)
        .await
        .unwrap();
    assert!(!exists);
}

#[tokio::test]
#[serial]
async fn body_id_cannot_differ_from_the_path_id() {
    let (state, raw, other, target) = setup().await;
    let app = init_app!(state);

    // Admin role grant: the body `user_id` cannot redirect the grant to another account.
    let role_id: i32 = sqlx::query_scalar("SELECT id FROM roles ORDER BY id LIMIT 1")
        .fetch_one(&raw)
        .await
        .unwrap();
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/users/{target}/roles/"))
        .insert_header(bearer(admin()))
        .set_json(json!({ "role_id": role_id, "user_id": other }))
        .to_request();
    assert_eq!(status!(app, req), StatusCode::BAD_REQUEST);
    let granted: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM user_roles WHERE user_id = $1 AND role_id = $2)",
    )
    .bind(other)
    .bind(role_id)
    .fetch_one(&raw)
    .await
    .unwrap();
    assert!(!granted);
}

#[tokio::test]
#[serial]
async fn sessions_of_another_user_are_not_listed() {
    let (state, raw, caller, target) = setup().await;
    let app = init_app!(state);
    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, device_info, ip_address, expires_at) \
         VALUES ($1, $2, 'deny-test', '127.0.0.1', NOW() + INTERVAL '1 hour')",
    )
    .bind(target)
    .bind(format!("deny-{}", uuid::Uuid::new_v4()))
    .execute(&raw)
    .await
    .unwrap();

    for uri in ["/api/v1/sessions/", "/api/v1/sessions/history"] {
        let req = test::TestRequest::get()
            .uri(uri)
            .insert_header(bearer(caller))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK, "{uri}");
        let body: Value = test::read_body_json(resp).await;
        assert!(
            !body.to_string().contains("deny-test"),
            "{uri} leaks another user's session: {body}"
        );
    }
}
