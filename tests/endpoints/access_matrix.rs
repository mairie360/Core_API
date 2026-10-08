//! MAIR-288: `access-matrix.yaml` covers every operation of the `OpenAPI`, and the API answers as
//! it says. Each non-public operation is called as a user of each role (and as the owner or a
//! member of the instance when the matrix grants it): an allowed caller never gets 401 / 403,
//! any other caller gets 403, and nobody gets in without a session.

use crate::common::get_raw_pool;
use actix_web::{http::StatusCode, test, web, App};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::swagger::ApiDoc;
use core_api::endpoints::{config, health, public_config, ready};
use core_api::session_jwt::generate_session_jwt;
use mairie360_api_lib::password::hash_password;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ADMIN_ID};
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde::Deserialize;
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::{PgPool, Row};
use std::collections::{BTreeMap, BTreeSet};
use utoipa::OpenApi;
use uuid::Uuid;

const MATRIX: &str = include_str!("../../access-matrix.yaml");
const ROLES: [&str; 5] = ["Admin", "Maire", "Responsable", "User", "Guest"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Matrix {
    version: u32,
    api: String,
    roles: Vec<String>,
    operations: BTreeMap<String, Operation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    access: String,
    #[serde(default)]
    roles: Vec<String>,
    ownership: Option<String>,
    #[serde(default)]
    personal: Vec<String>,
    #[serde(default)]
    token: bool,
    #[allow(dead_code)]
    note: Option<String>,
}

fn matrix() -> Matrix {
    yaml_serde::from_str(MATRIX).expect("access-matrix.yaml is valid")
}

/// The spec the API serves, as JSON.
fn spec() -> Value {
    serde_json::to_value(ApiDoc::openapi()).unwrap()
}

/// "METHOD /path" of every operation of the spec.
fn spec_operations(spec: &Value) -> BTreeSet<String> {
    let mut operations = BTreeSet::new();
    for (path, item) in spec["paths"].as_object().unwrap() {
        for method in ["get", "post", "put", "patch", "delete"] {
            if item.get(method).is_some() {
                operations.insert(format!("{} {path}", method.to_uppercase()));
            }
        }
    }
    operations
}

#[core::prelude::v1::test]
fn the_matrix_covers_every_operation_of_the_spec() {
    let matrix = matrix();
    assert_eq!((matrix.version, matrix.api.as_str()), (1, "core"));
    assert_eq!(matrix.roles, ROLES);
    let spec = spec_operations(&spec());
    let listed: BTreeSet<String> = matrix.operations.keys().cloned().collect();
    let missing: Vec<_> = spec.difference(&listed).collect();
    let unknown: Vec<_> = listed.difference(&spec).collect();
    assert!(
        missing.is_empty(),
        "operations of the OpenAPI missing from access-matrix.yaml: {missing:?}"
    );
    assert!(
        unknown.is_empty(),
        "operations of access-matrix.yaml that the OpenAPI does not have: {unknown:?}"
    );
    for (name, op) in &matrix.operations {
        assert!(
            ["public", "authenticated", "admin"].contains(&op.access.as_str()),
            "{name}: access"
        );
        assert!(
            op.roles.iter().all(|r| ROLES.contains(&r.as_str())),
            "{name}: unknown role"
        );
        assert!(
            op.ownership
                .as_deref()
                .is_none_or(|o| ["self", "owner", "member"].contains(&o)),
            "{name}: ownership"
        );
        if op.access == "admin" {
            assert_eq!(op.roles, ["Admin"], "{name}: the admin scope is Admin only");
        }
        if op.access == "authenticated" {
            assert!(
                !op.roles.is_empty() || op.ownership.is_some(),
                "{name}: nobody may call it"
            );
        }
        assert!(
            op.personal.iter().all(|f| !f.is_empty()),
            "{name}: personal"
        );
    }
}

/// Same wiring as `main.rs`.
macro_rules! init_app {
    ($state:expr) => {
        test::init_service(
            App::new()
                .app_data($state.clone())
                .service(health::health)
                .service(ready::ready)
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

/// A user with `role` (the schema also gives every account the Guest role).
async fn user_with_role(raw: &PgPool, role: &str) -> u64 {
    let id: i32 = sqlx::query(
        "INSERT INTO users (first_name, last_name, email, password, status) \
         VALUES ('Matrix', $1, $2, $3, 'active') RETURNING id",
    )
    .bind(role)
    .bind(format!("matrix.{}@example.com", Uuid::new_v4()))
    .bind(hash_password("Matrix!Pass123").unwrap())
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id");
    sqlx::query(
        "INSERT INTO user_roles (user_id, role_id) SELECT $1, id FROM roles WHERE lower(name) = lower($2) \
         ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(role)
    .execute(raw)
    .await
    .unwrap();
    u64::try_from(id).unwrap()
}

/// A fresh JWT of a fresh session of `user_id` (a call may log the session out), and the
/// session's refresh token.
async fn session(state: &AppState, user_id: u64) -> (String, String) {
    let session_id = Uuid::new_v4();
    let refresh_token = format!("matrix_{session_id}");
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            user_id,
            &core_api::refresh_token::hash(&refresh_token),
            "matrix",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();
    (
        format!(
            "Bearer {}",
            generate_session_jwt(user_id, session_id).unwrap()
        ),
        refresh_token,
    )
}

async fn bearer(state: &AppState, user_id: u64) -> String {
    session(state, user_id).await.0
}

/// Fresh instances for one call: a group owned by `owner` (who is a member), an ACL entry on it,
/// a target user, a deletable role.
struct Fixtures {
    group: i32,
    acl: i32,
    target: i32,
    role: i32,
}

async fn fixtures(raw: &PgPool, owner: u64) -> Fixtures {
    let owner = i32::try_from(owner).unwrap();
    let marker = Uuid::new_v4().simple().to_string();
    let group: i32 = sqlx::query(
        "INSERT INTO groups (owner_id, name, description) VALUES ($1, $2, '') RETURNING id",
    )
    .bind(owner)
    .bind(format!("matrix-{}", &marker[..20]))
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id");
    sqlx::query(
        "INSERT INTO group_members (group_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(group)
    .bind(owner)
    .execute(raw)
    .await
    .unwrap();
    let target = i32::try_from(user_with_role(raw, "User").await).unwrap();
    let acl: i32 = sqlx::query(
        "INSERT INTO access_control (user_id, resource_id, resource_instance_id, permission_id) \
         SELECT $1, res.id, $2, p.id FROM resources res JOIN permissions p ON p.resource_id = res.id \
         WHERE res.name = 'groups' AND p.action = 'read' RETURNING id",
    )
    .bind(target)
    .bind(group)
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id");
    let role: i32 = sqlx::query(
        "INSERT INTO roles (name, description, can_be_deleted) VALUES ($1, '', true) RETURNING id",
    )
    .bind(format!("matrix-{}", &marker[..20]))
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id");
    Fixtures {
        group,
        acl,
        target,
        role,
    }
}

/// The request of one operation on the fixtures: path parameters, body (the example of the spec,
/// with the fixtures' ids where the handler checks them).
fn request(spec: &Value, name: &str, f: &Fixtures, refresh_token: &str) -> test::TestRequest {
    let (method, path) = name.split_once(' ').unwrap();
    let mut uri = path
        .replace("{group_id}", &f.group.to_string())
        .replace("{user_id}", &f.target.to_string())
        .replace("{userId}", &f.target.to_string())
        .replace("{roleId}", &f.role.to_string());
    uri = if path.starts_with("/api/v1/admin/roles/") {
        uri.replace("{id}", &f.role.to_string())
    } else if path.starts_with("/api/v1/ressources/") {
        format!(
            "{}?ressource_type=groups",
            uri.replace("{id}", &f.group.to_string())
        )
    } else {
        uri.replace("{id}", &f.target.to_string())
    };
    let example = spec["paths"][path][method.to_lowercase()]["requestBody"]["content"]
        ["application/json"]["example"]
        .clone();
    let body = match path {
        "/api/v1/ressources/add_access" => Some(
            json!({ "user_id": f.target, "resource_id": f.group, "ressource_type": "groups", "access_type": "Read" }),
        ),
        "/api/v1/ressources/remove_access" => Some(json!({ "access_id": f.acl })),
        "/api/v1/groups/{group_id}/users/" if method == "POST" => {
            Some(json!({ "user_id": f.target }))
        }
        "/api/v1/sessions/revoke" => Some(json!({ "refresh_token": refresh_token })),
        _ if example.is_null() => None,
        _ => Some(example),
    };
    let req = match method {
        "GET" => test::TestRequest::get(),
        "POST" => test::TestRequest::post(),
        "PUT" => test::TestRequest::put(),
        "PATCH" => test::TestRequest::patch(),
        "DELETE" => test::TestRequest::delete(),
        other => panic!("{other}"),
    }
    .uri(&uri);
    match body {
        Some(body) => req.set_json(body),
        None => req,
    }
}

macro_rules! status {
    ($app:expr, $req:expr) => {
        match test::try_call_service(&$app, $req).await {
            Ok(resp) => resp.status(),
            Err(err) => err.as_response_error().status_code(),
        }
    };
}

#[tokio::test]
#[serial]
async fn every_role_gets_what_the_matrix_says() {
    std::env::set_var("JWT_SECRET", "access_matrix_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
    std::env::remove_var("DIRECTORY_GUEST_ACCESS");
    let (_container, host) = get_shared_db().await;
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), host.clone()).await);
    let raw = get_raw_pool(host.clone()).await;
    let app = init_app!(state);
    let spec = spec();

    let mut callers: Vec<(&str, u64)> =
        vec![("Admin", u64::try_from(*ADMIN_ID.get().unwrap()).unwrap())];
    for role in &ROLES[1..] {
        callers.push((role, user_with_role(&raw, role).await));
    }
    // Owner (and member) of the fixtures' group: a plain User.
    let owner = user_with_role(&raw, "User").await;

    let mut failures = Vec::new();
    for (name, op) in &matrix().operations {
        if op.access == "public" {
            let f = fixtures(&raw, owner).await;
            let got = status!(app, request(&spec, name, &f, "").to_request());
            // A wrong one-time or refresh token answers 401 / 403: only the route is checked.
            let refused = got == StatusCode::UNAUTHORIZED || got == StatusCode::FORBIDDEN;
            if (refused && !op.token)
                || got == StatusCode::NOT_FOUND
                || got == StatusCode::METHOD_NOT_ALLOWED
            {
                failures.push(format!("{name}: public, got {got} without a session"));
            }
            continue;
        }
        let f = fixtures(&raw, owner).await;
        let got = status!(app, request(&spec, name, &f, "").to_request());
        if got != StatusCode::UNAUTHORIZED {
            failures.push(format!("{name}: got {got} without a session, expected 401"));
        }
        let ownership = op.ownership.as_deref();
        let mut cases: Vec<(String, u64, bool)> = callers
            .iter()
            .map(|(role, id)| {
                (
                    role.to_string(),
                    *id,
                    ownership == Some("self") || op.roles.iter().any(|r| r == role),
                )
            })
            .collect();
        if matches!(ownership, Some("owner" | "member")) {
            cases.push(("owner of the instance".to_string(), owner, true));
        }
        for (who, user, allowed) in cases {
            let f = fixtures(&raw, owner).await;
            let (token, refresh_token) = session(&state, user).await;
            let got = status!(
                app,
                request(&spec, name, &f, &refresh_token)
                    .insert_header(("Authorization", token))
                    .to_request()
            );
            let refused = got == StatusCode::UNAUTHORIZED || got == StatusCode::FORBIDDEN;
            if allowed && refused {
                failures.push(format!(
                    "{name}: {who} is allowed by the matrix but got {got}"
                ));
            }
            if !allowed && got != StatusCode::FORBIDDEN {
                failures.push(format!(
                    "{name}: {who} is not allowed by the matrix but got {got}, expected 403"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "the API does not answer as access-matrix.yaml says:\n{}",
        failures.join("\n")
    );
}

/// `DIRECTORY_GUEST_ACCESS=none` closes the directory to the agents who only hold the Guest role;
/// they still read their own record, and the other roles are not affected.
#[tokio::test]
#[serial]
async fn the_directory_can_be_closed_to_guests() {
    std::env::set_var("JWT_SECRET", "access_matrix_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
    let (_container, host) = get_shared_db().await;
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), host.clone()).await);
    let raw = get_raw_pool(host.clone()).await;
    let app = init_app!(state);
    let guest = user_with_role(&raw, "Guest").await;
    let agent = user_with_role(&raw, "User").await;
    let get = |uri: String, token: String| {
        test::TestRequest::get()
            .uri(&uri)
            .insert_header(("Authorization", token))
            .to_request()
    };

    std::env::set_var("DIRECTORY_GUEST_ACCESS", "none");
    let directory = status!(
        app,
        get("/api/v1/user/".into(), bearer(&state, guest).await)
    );
    let other = status!(
        app,
        get(
            format!("/api/v1/user/{agent}/"),
            bearer(&state, guest).await
        )
    );
    let own = status!(
        app,
        get(
            format!("/api/v1/user/{guest}/"),
            bearer(&state, guest).await
        )
    );
    let agent_directory = status!(
        app,
        get("/api/v1/user/".into(), bearer(&state, agent).await)
    );
    std::env::remove_var("DIRECTORY_GUEST_ACCESS");
    let reopened = status!(
        app,
        get("/api/v1/user/".into(), bearer(&state, guest).await)
    );

    assert_eq!(directory, StatusCode::FORBIDDEN);
    assert_eq!(other, StatusCode::FORBIDDEN);
    assert_eq!(own, StatusCode::OK);
    assert_eq!(agent_directory, StatusCode::OK);
    assert_eq!(reopened, StatusCode::OK, "open to every agent by default");
}
