//! MAIR-524: Redis read cache of the directory (`GET /api/v1/user/`).
//!
//! Every test runs against a Redis enforcing the chart's ACL, connected as `core-api` (so the
//! entries live under `core-api:read-cache:directory:*`), and isolates its rows in the shared
//! database with a unique marker used as the `search` filter. A cache hit is shown by changing a
//! row with raw SQL behind the API: the cached page keeps the old value until a write through
//! the API invalidates the namespace.

use crate::common::acl_redis::AclRedis;
use crate::common::{
    core_db_url, get_pool, get_raw_pool, users::create_user, users::unique_marker,
};
use actix_web::http::Method;
use actix_web::{http::StatusCode, test, web, App};
use core_api::database::users::list_directory::DIRECTORY_CACHE_TTL_SECONDS;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ADMIN_ID};
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use redis::Commands;
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::{PgPool, Row};

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as the other endpoint tests: they share the test binary.
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

struct Env {
    // Kept alive for the duration of the test.
    redis: AclRedis,
    state: web::Data<AppState>,
    pool: SmartDatabase,
    raw: PgPool,
}

// `AppState::new` holds non-`Send` tracing values across awaits; tests run on one thread.
#[allow(clippy::future_not_send)]
async fn env() -> Env {
    std::sync::LazyLock::force(&INIT);
    // The key prefix must come from the URL's user, like in the chart.
    std::env::remove_var("REDIS_KEY_PREFIX");
    std::env::remove_var("REDIS_USERNAME");
    let (_container, host) = get_shared_db().await;
    let redis = AclRedis::start().await;
    let state = web::Data::new(AppState::new(redis.url_as("core-api"), core_db_url().await).await);
    Env {
        redis,
        state,
        pool: get_pool().await,
        raw: get_raw_pool(host.clone()).await,
    }
}

fn admin() -> i32 {
    *ADMIN_ID.get().unwrap()
}

fn jwt(user_id: i32) -> String {
    generate_jwt(&user_id.to_string(), "").expect("test JWT")
}

// actix test services hold `Rc`s, so this future is `!Send`; tests run it on one thread anyway.
#[allow(clippy::future_not_send)]
async fn call(
    env: &Env,
    caller: i32,
    method: Method,
    path: &str,
    payload: Option<Value>,
) -> (StatusCode, String) {
    let app = test::init_service(
        App::new()
            .app_data(env.state.clone())
            .configure(public_config)
            .service(web::scope("/api").wrap(JwtMiddleware).configure(config)),
    )
    .await;
    let mut req = test::TestRequest::default()
        .method(method)
        .uri(path)
        .insert_header(("Authorization", format!("Bearer {}", jwt(caller))));
    if let Some(payload) = payload {
        req = req.set_json(payload);
    }
    match test::try_call_service(&app, req.to_request()).await {
        Ok(resp) => {
            let status = resp.status();
            let body = String::from_utf8(test::read_body(resp).await.to_vec()).unwrap();
            (status, body)
        }
        Err(error) => (error.as_response_error().status_code(), error.to_string()),
    }
}

/// `GET /api/v1/user/?<query>` as `caller`, the `users` array.
#[allow(clippy::future_not_send)]
async fn directory(env: &Env, caller: i32, query: &str) -> Vec<Value> {
    let (status, body) = call(
        env,
        caller,
        Method::GET,
        &format!("/api/v1/user/?{query}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    serde_json::from_str::<Value>(&body).unwrap()["users"]
        .as_array()
        .unwrap()
        .clone()
}

/// The single directory entry of `search=<marker>`.
#[allow(clippy::future_not_send)]
async fn only_entry(env: &Env, marker: &str) -> Value {
    let users = directory(env, admin(), &format!("search={marker}")).await;
    assert_eq!(users.len(), 1, "{users:?}");
    users[0].clone()
}

async fn rename_behind_the_api(raw: &PgPool, user_id: i32, first_name: &str) {
    sqlx::query("UPDATE users SET first_name = $1 WHERE id = $2")
        .bind(first_name)
        .bind(user_id)
        .execute(raw)
        .await
        .unwrap();
}

async fn new_role(raw: &PgPool, name: &str) -> i32 {
    sqlx::query(
        "INSERT INTO roles (name, description, can_be_deleted) VALUES ($1, 'test', true) \
         RETURNING id",
    )
    .bind(name)
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id")
}

fn names(entry: &Value) -> Vec<String> {
    entry["roles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|role| role.as_str().unwrap().to_string())
        .collect()
}

fn group_ids(entry: &Value) -> Vec<i64> {
    entry["group_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_i64().unwrap())
        .collect()
}

#[tokio::test]
#[serial]
async fn directory_page_is_cached_under_the_acl_until_an_admin_patch() {
    let env = env().await;
    let marker = unique_marker("dircache");
    let user = create_user(&env.pool, "Cached", &marker).await;

    assert_eq!(only_entry(&env, &marker).await["first_name"], "Cached");
    rename_behind_the_api(&env.raw, user, "Behind").await;

    // Served from Redis: the raw change is not visible, whoever asks and however the filters
    // are spelled (trimmed search, ids as a set).
    assert_eq!(only_entry(&env, &marker).await["first_name"], "Cached");
    let other_caller = create_user(&env.pool, "Other", &unique_marker("dircache")).await;
    let page = directory(&env, other_caller, &format!("search={marker}")).await;
    assert_eq!(page[0]["first_name"], "Cached", "shared entry");
    let page = directory(&env, admin(), &format!("search=%20{marker}%20")).await;
    assert_eq!(page[0]["first_name"], "Cached", "search is trimmed");
    let by_ids = directory(&env, admin(), &format!("ids={user},{user}")).await;
    assert_eq!(
        by_ids[0]["first_name"], "Behind",
        "first read of these filters"
    );
    rename_behind_the_api(&env.raw, user, "Behind twice").await;
    let by_ids = directory(&env, admin(), &format!("ids={user}")).await;
    assert_eq!(by_ids[0]["first_name"], "Behind", "same id set, same entry");

    // The entries are written under the `core-api:` prefix with the cache TTL, never in clear.
    let mut redis = env.redis.admin();
    let keys: Vec<String> = redis.keys("core-api:read-cache:directory:*").unwrap();
    assert!(keys.iter().any(|key| key.contains(":shared:")), "{keys:?}");
    for key in keys.iter().filter(|key| key.contains(":shared:")) {
        assert!(!key.contains(&marker), "{key}");
        let ttl: i64 = redis.ttl(key).unwrap();
        assert!(
            ttl > 0 && ttl.unsigned_abs() <= DIRECTORY_CACHE_TTL_SECONDS,
            "{key}: {ttl}"
        );
    }
    let unprefixed: Vec<String> = redis.keys("read-cache:*").unwrap();
    assert!(unprefixed.is_empty(), "{unprefixed:?}");

    let (status, body) = call(
        &env,
        admin(),
        Method::PATCH,
        &format!("/api/v1/admin/users/{user}/"),
        Some(json!({ "first_name": "Patched" })),
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    assert_eq!(only_entry(&env, &marker).await["first_name"], "Patched");
    let by_ids = directory(&env, admin(), &format!("ids={user}")).await;
    assert_eq!(by_ids[0]["first_name"], "Patched");
}

#[tokio::test]
#[serial]
async fn patch_me_invalidates_the_directory() {
    let env = env().await;
    let marker = unique_marker("dircache");
    let user = create_user(&env.pool, "Self", &marker).await;

    assert_eq!(only_entry(&env, &marker).await["first_name"], "Self");
    rename_behind_the_api(&env.raw, user, "Behind").await;
    assert_eq!(only_entry(&env, &marker).await["first_name"], "Self");

    let (status, body) = call(
        &env,
        user,
        Method::PATCH,
        "/api/v1/user/me/",
        Some(json!({ "first_name": "Renamed" })),
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    assert_eq!(only_entry(&env, &marker).await["first_name"], "Renamed");
}

#[tokio::test]
#[serial]
async fn role_grant_revoke_rename_and_delete_invalidate_the_directory() {
    let env = env().await;
    let marker = unique_marker("dircache");
    let user = create_user(&env.pool, "Roles", &marker).await;
    let role_name = unique_marker("Role");
    let role = new_role(&env.raw, &role_name).await;

    assert!(!names(&only_entry(&env, &marker).await).contains(&role_name));

    let (status, body) = call(
        &env,
        admin(),
        Method::POST,
        &format!("/api/v1/admin/users/{user}/roles/"),
        Some(json!({ "role_id": role })),
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    assert!(names(&only_entry(&env, &marker).await).contains(&role_name));

    let renamed = unique_marker("Renamed");
    let (status, body) = call(
        &env,
        admin(),
        Method::PATCH,
        &format!("/api/v1/admin/roles/{role}"),
        Some(json!({ "name": renamed })),
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    let roles = names(&only_entry(&env, &marker).await);
    assert!(roles.contains(&renamed), "{roles:?}");
    assert!(!roles.contains(&role_name), "{roles:?}");

    let replaced = unique_marker("Replaced");
    let (status, body) = call(
        &env,
        admin(),
        Method::PUT,
        &format!("/api/v1/admin/roles/{role}"),
        Some(json!({ "name": replaced, "description": "test" })),
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    assert!(names(&only_entry(&env, &marker).await).contains(&replaced));

    let (status, body) = call(
        &env,
        admin(),
        Method::DELETE,
        &format!("/api/v1/admin/users/{user}/roles/{role}"),
        None,
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    assert!(!names(&only_entry(&env, &marker).await).contains(&replaced));

    // Granted again, then the role itself is deleted: the cascade empties the entry.
    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
        .bind(user)
        .bind(role)
        .execute(&env.raw)
        .await
        .unwrap();
    let (status, body) = call(
        &env,
        admin(),
        Method::PATCH,
        &format!("/api/v1/admin/roles/{role}"),
        Some(json!({ "description": "refresh" })),
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    assert!(names(&only_entry(&env, &marker).await).contains(&replaced));
    let (status, body) = call(
        &env,
        admin(),
        Method::DELETE,
        &format!("/api/v1/admin/roles/{role}"),
        None,
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    assert!(!names(&only_entry(&env, &marker).await).contains(&replaced));
}

#[tokio::test]
#[serial]
async fn group_writes_invalidate_the_directory() {
    let env = env().await;
    let marker = unique_marker("dircache");
    let user = create_user(&env.pool, "Member", &marker).await;
    let owner = create_user(&env.pool, "Owner", &unique_marker("dircache")).await;
    let owner_query = format!("ids={owner}");

    assert!(group_ids(&only_entry(&env, &marker).await).is_empty());
    assert!(group_ids(&directory(&env, owner, &owner_query).await[0]).is_empty());

    // The creation trigger adds the owner as a member.
    let (status, body) = call(
        &env,
        owner,
        Method::POST,
        "/api/v1/groups/",
        Some(json!({ "name": unique_marker("group"), "description": "test" })),
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    let group = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap();
    assert_eq!(
        group_ids(&directory(&env, owner, &owner_query).await[0]),
        vec![group]
    );

    let (status, body) = call(
        &env,
        owner,
        Method::POST,
        &format!("/api/v1/groups/{group}/users/"),
        Some(json!({ "user_id": user })),
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    assert_eq!(group_ids(&only_entry(&env, &marker).await), vec![group]);
    let scoped = directory(&env, owner, &format!("group_ids={group}&search={marker}")).await;
    assert_eq!(scoped.len(), 1);

    let (status, body) = call(
        &env,
        owner,
        Method::DELETE,
        &format!("/api/v1/groups/{group}/users/{user}/"),
        None,
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    assert!(group_ids(&only_entry(&env, &marker).await).is_empty());
    let scoped = directory(&env, owner, &format!("group_ids={group}&search={marker}")).await;
    assert!(scoped.is_empty(), "{scoped:?}");

    let (status, body) = call(
        &env,
        owner,
        Method::DELETE,
        &format!("/api/v1/groups/{group}/"),
        None,
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    assert!(group_ids(&directory(&env, owner, &owner_query).await[0]).is_empty());
}

#[tokio::test]
#[serial]
async fn admin_creation_and_archive_invalidate_the_directory() {
    let env = env().await;
    let marker = unique_marker("dircache");

    // An empty page is cached too, and replaced as soon as the account is created.
    assert!(directory(&env, admin(), &format!("search={marker}"))
        .await
        .is_empty());
    let (status, body) = call(
        &env,
        admin(),
        Method::POST,
        "/api/v1/admin/users/",
        Some(json!({
            "first_name": "Created",
            "last_name": marker,
            "email": format!("created.{marker}@example.com"),
            "password": "a-long-enough-password"
        })),
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    let user = only_entry(&env, &marker).await["id"].as_i64().unwrap();
    assert_eq!(
        directory(&env, admin(), &format!("ids={user}")).await.len(),
        1
    );

    // After the archive, no cached page still shows the account.
    let (status, body) = call(
        &env,
        admin(),
        Method::DELETE,
        &format!("/api/v1/admin/users/{user}/"),
        None,
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    assert!(directory(&env, admin(), &format!("search={marker}"))
        .await
        .is_empty());
    assert!(directory(&env, admin(), &format!("ids={user}"))
        .await
        .is_empty());
}
