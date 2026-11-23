//! MAIR-916 / MAIR-923 (epic MAIR-824): modules of a role, services of the mairie and the service of
//! an agent, colour and project count of a group.

use crate::common::{core_db_url, get_raw_pool};
use actix_web::{http::StatusCode, test, web, App};
use base64::{engine::general_purpose, Engine as _};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::second_factor::SecretCipher;
use core_api::session_jwt::generate_session_jwt;
use mairie360_api_lib::test_setup::queries_setup::ADMIN_ID;
use mairie360_api_lib::test_setup::redis_setup::get_shared_redis;
use mairie360_api_lib::{password::hash_password, security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use uuid::Uuid;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

const PASSWORD: &str = "Correct!Pass123";

macro_rules! init_app {
    ($state:expr, $cipher:expr) => {
        test::init_service(
            App::new()
                .app_data($state.clone())
                .app_data($cipher.clone())
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
    ($app:expr, $req:expr) => {{
        match test::try_call_service(&$app, $req.to_request()).await {
            Ok(resp) => {
                let status = resp.status();
                let bytes = test::read_body(resp).await;
                let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                (status, body, String::from_utf8_lossy(&bytes).to_string())
            }
            Err(err) => (
                err.as_response_error().status_code(),
                Value::Null,
                err.to_string(),
            ),
        }
    }};
}

struct Env {
    state: web::Data<AppState>,
    cipher: web::Data<SecretCipher>,
    raw: PgPool,
}

async fn env() -> Env {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let redis = get_shared_redis().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let cipher = web::Data::new(
        SecretCipher::from_base64(&general_purpose::STANDARD.encode([7u8; 32])).unwrap(),
    );
    Env {
        state,
        cipher,
        raw: get_raw_pool(host).await,
    }
}

/// Inserts an active account with `PASSWORD` and returns `(id, email)`.
async fn create_account(raw: &PgPool) -> (u64, String) {
    let email = format!("mair923.{}@example.com", Uuid::new_v4());
    let id: i32 = sqlx::query(
        "INSERT INTO users (first_name, last_name, email, password, status, first_connect, \
         is_archived) VALUES ('Session', 'Holder', $1, $2, 'active', false, false) RETURNING id",
    )
    .bind(&email)
    .bind(hash_password(PASSWORD).unwrap())
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id");
    (u64::try_from(id).unwrap(), email)
}

/// Opens a session for `user_id` as the login does; returns `(session id, Authorization header)`.
async fn open_session(state: &AppState, user_id: u64) -> (Uuid, (&'static str, String)) {
    let session_id = Uuid::new_v4();
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            user_id,
            &core_api::refresh_token::hash(&format!("mair923_{}", Uuid::new_v4())),
            "Firefox 140 sur Linux",
            std::net::IpAddr::from([203, 0, 113, 24]),
        ))
        .await
        .unwrap();
    let jwt = generate_session_jwt(user_id, session_id, "").unwrap();
    (session_id, ("Authorization", format!("Bearer {jwt}")))
}

async fn admin(state: &AppState) -> (&'static str, String) {
    open_session(state, u64::try_from(*ADMIN_ID.get().unwrap()).unwrap())
        .await
        .1
}

fn as_admin(req: test::TestRequest, admin: &(&'static str, String)) -> test::TestRequest {
    req.insert_header(admin.clone())
}

#[actix_web::test]
async fn mair_916_a_role_keeps_its_modules_and_the_admin_role_keeps_the_administration() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin = admin(&env.state).await;
    let name = format!("modules-{}", Uuid::new_v4());

    let (status, _, text) = call!(
        app,
        as_admin(test::TestRequest::post().uri("/api/v1/admin/roles/"), &admin).set_json(json!({
            "name": name, "description": "", "can_be_deleted": true, "modules": ["calendar", "dashboard"]
        }))
    );
    assert_eq!(status, StatusCode::OK, "{text}");
    let (_, body, _) = call!(
        app,
        as_admin(test::TestRequest::get().uri("/api/v1/admin/roles/"), &admin)
    );
    let role = body["roles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|role| role["name"] == name)
        .cloned()
        .unwrap();
    assert_eq!(role["modules"], json!(["dashboard", "calendar"]));
    let id = role["id"].as_u64().unwrap();

    let (status, _, _) = call!(
        app,
        as_admin(
            test::TestRequest::patch().uri(&format!("/api/v1/admin/roles/{id}")),
            &admin
        )
        .set_json(json!({ "modules": ["files"] }))
    );
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _, _) = call!(
        app,
        as_admin(
            test::TestRequest::patch().uri(&format!("/api/v1/admin/roles/{id}")),
            &admin
        )
        .set_json(json!({ "modules": ["projects"] }))
    );
    assert_eq!(status, StatusCode::OK);
    let modules: Vec<String> =
        sqlx::query_scalar("SELECT module FROM role_modules WHERE role_id = $1")
            .bind(i32::try_from(id).unwrap())
            .fetch_all(&env.raw)
            .await
            .unwrap();
    assert_eq!(modules, vec!["projects".to_string()]);

    let admin_role: i32 = sqlx::query_scalar("SELECT id FROM roles WHERE name = 'Admin'")
        .fetch_one(&env.raw)
        .await
        .unwrap();
    let (status, _, text) = call!(
        app,
        as_admin(
            test::TestRequest::patch().uri(&format!("/api/v1/admin/roles/{admin_role}")),
            &admin
        )
        .set_json(json!({ "modules": ["dashboard"] }))
    );
    assert_eq!(status, StatusCode::FORBIDDEN, "{text}");

    let (status, body, _) = call!(
        app,
        as_admin(test::TestRequest::get().uri("/api/v1/user/me/"), &admin)
    );
    assert_eq!(status, StatusCode::OK);
    assert!(body["modules"]
        .as_array()
        .unwrap()
        .contains(&json!("admin")));
}

#[actix_web::test]
async fn mair_923_services_and_the_service_of_an_agent() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin = admin(&env.state).await;
    let (user_id, email) = create_account(&env.raw).await;
    let name = format!("Voirie {}", Uuid::new_v4());

    let (status, service, text) = call!(
        app,
        as_admin(
            test::TestRequest::post().uri("/api/v1/admin/services/"),
            &admin
        )
        .set_json(json!({ "name": name }))
    );
    assert_eq!(status, StatusCode::CREATED, "{text}");
    assert_eq!(service["agents"], 0);
    let service_id = service["id"].as_u64().unwrap();

    let (status, _, _) = call!(
        app,
        as_admin(
            test::TestRequest::post().uri("/api/v1/admin/services/"),
            &admin
        )
        .set_json(json!({ "name": name.to_uppercase() }))
    );
    assert_eq!(status, StatusCode::CONFLICT);

    let set_service = |service: Value| {
        as_admin(
            test::TestRequest::put().uri(&format!("/api/v1/admin/users/{user_id}/service")),
            &admin,
        )
        .set_json(json!({ "service_id": service }))
    };
    let (status, _, text) = call!(app, set_service(json!(service_id)));
    assert_eq!(status, StatusCode::NO_CONTENT, "{text}");
    let (status, _, _) = call!(app, set_service(json!(99_999_999)));
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = call!(
        app,
        as_admin(
            test::TestRequest::put().uri("/api/v1/admin/users/99999999/service"),
            &admin
        )
        .set_json(json!({ "service_id": service_id }))
    );
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, record, _) = call!(
        app,
        as_admin(
            test::TestRequest::get().uri(&format!("/api/v1/admin/users/{user_id}/")),
            &admin
        )
    );
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        record["user"]["service"],
        json!({ "id": service_id, "name": name })
    );

    // The admin search finds the agent by the name of their service.
    let (status, list, _) = call!(
        app,
        as_admin(
            test::TestRequest::get().uri(&format!(
                "/api/v1/admin/users/?search={}",
                name.split(' ').nth(1).unwrap()
            )),
            &admin
        )
    );
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["users"][0]["email"], email);
    assert_eq!(list["users"][0]["service"]["name"], name);

    let (_, services, _) = call!(
        app,
        as_admin(
            test::TestRequest::get().uri("/api/v1/admin/services/"),
            &admin
        )
    );
    let listed = services["services"]
        .as_array()
        .unwrap()
        .iter()
        .find(|service| service["id"] == service_id)
        .cloned()
        .unwrap();
    assert_eq!(listed["agents"], 1);

    let (status, renamed, _) = call!(
        app,
        as_admin(
            test::TestRequest::patch().uri(&format!("/api/v1/admin/services/{service_id}")),
            &admin
        )
        .set_json(json!({ "name": format!("{name} bis") }))
    );
    assert_eq!(status, StatusCode::OK);
    assert_eq!(renamed["name"], format!("{name} bis"));

    let (status, _, _) = call!(
        app,
        as_admin(
            test::TestRequest::delete().uri(&format!("/api/v1/admin/services/{service_id}")),
            &admin
        )
    );
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, record, _) = call!(
        app,
        as_admin(
            test::TestRequest::get().uri(&format!("/api/v1/admin/users/{user_id}/")),
            &admin
        )
    );
    assert!(record["user"]["service"].is_null());
    let (status, _, _) = call!(
        app,
        as_admin(
            test::TestRequest::delete().uri(&format!("/api/v1/admin/services/{service_id}")),
            &admin
        )
    );
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn mair_923_a_group_has_a_colour_of_the_palette_and_counts_its_projects() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin = admin(&env.state).await;

    let (status, created, text) = call!(
        app,
        as_admin(test::TestRequest::post().uri("/api/v1/groups/"), &admin).set_json(json!({
            "name": format!("Couleur {}", Uuid::new_v4()), "description": "", "color": "teal"
        }))
    );
    assert_eq!(status, StatusCode::OK, "{text}");
    let id = created["id"].as_u64().unwrap();
    let group_id = i32::try_from(id).unwrap();

    let (status, _, _) = call!(
        app,
        as_admin(test::TestRequest::post().uri("/api/v1/groups/"), &admin).set_json(json!({
            "name": format!("Rouge {}", Uuid::new_v4()), "description": "", "color": "red"
        }))
    );
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let color: Option<String> = sqlx::query_scalar("SELECT color FROM groups WHERE id = $1")
        .bind(group_id)
        .fetch_one(&env.raw)
        .await
        .unwrap();
    assert_eq!(color.as_deref(), Some("teal"));

    let project_id: i32 = sqlx::query_scalar(
        "INSERT INTO projects (name, owner_id) VALUES ('Projet couleur', $1) RETURNING id",
    )
    .bind(*ADMIN_ID.get().unwrap())
    .fetch_one(&env.raw)
    .await
    .unwrap();
    sqlx::query("INSERT INTO project_groups (project_id, group_id) VALUES ($1, $2)")
        .bind(project_id)
        .bind(group_id)
        .execute(&env.raw)
        .await
        .unwrap();

    let patch = |body: Value| {
        as_admin(
            test::TestRequest::patch().uri(&format!("/api/v1/groups/{id}/")),
            &admin,
        )
        .set_json(body)
    };
    let (status, group, text) = call!(app, patch(json!({ "color": "orange" })));
    assert_eq!(status, StatusCode::OK, "{text}");
    assert_eq!(group["color"], "orange");
    assert_eq!(group["project_count"], 1);

    let (status, _, _) = call!(app, patch(json!({ "color": "Orange" })));
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, group, _) = call!(app, patch(json!({ "color": null })));
    assert_eq!(status, StatusCode::OK);
    assert!(group["color"].is_null());

    let (status, group, _) = call!(
        app,
        patch(json!({ "description": "Toujours sans couleur" }))
    );
    assert_eq!(status, StatusCode::OK);
    assert!(group["color"].is_null());
}
