//! MAIR-1228: `GET /api/v1/search`, Core's part of the global search of the shared header
//! (agents only). Each test isolates its rows with a unique marker used as the term.

use crate::common::{
    core_db_url, get_pool, get_raw_pool, own_db_url,
    users::{archive_user, create_user, unique_marker},
};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::Value;
use serial_test::serial;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as the other endpoint tests: they share the test binary.
    mairie360_api_lib::test_setup::http_setup::init_test_jwt_env();
    std::env::set_var("JWT_TIMEOUT", "3600");
});

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

macro_rules! search {
    ($app:expr, $caller:expr, $query:expr) => {{
        let request = test::TestRequest::get()
            .uri(&format!("/api/v1/search{}", $query))
            .insert_header((
                "Authorization",
                format!(
                    "Bearer {}",
                    generate_jwt(&$caller.to_string(), "user").unwrap()
                ),
            ));
        let resp = test::call_service(&$app, request.to_request()).await;
        let status = resp.status();
        let body = String::from_utf8_lossy(&test::read_body(resp).await).to_string();
        (status, body)
    }};
}

fn items(body: &str) -> Vec<Value> {
    serde_json::from_str::<Value>(body).expect("JSON body")["items"]
        .as_array()
        .expect("items")
        .clone()
}

#[tokio::test]
#[serial(process_env)]
async fn mair_1228_finds_an_agent_then_no_longer_once_archived() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let pool = get_pool().await;
    let app = init_app!(state);
    let marker = unique_marker("search");
    let caller = create_user(&pool, "Caller", &unique_marker("caller")).await;
    let agent = create_user(&pool, "Searched", &marker).await;

    let (status, body) = search!(app, caller, format!("?q={marker}"));
    assert_eq!(status, StatusCode::OK, "{body}");
    let found = items(&body);
    assert_eq!(found.len(), 1, "{body}");
    assert_eq!(found[0]["kind"], "agent");
    assert_eq!(found[0]["id"], agent);
    assert_eq!(found[0]["title"], format!("Searched {marker}"));
    assert!(found[0]["excerpt"]
        .as_str()
        .unwrap()
        .contains(&marker.to_lowercase()));
    assert!(found[0]["parent_id"].is_null());
    assert!(found[0]["updated_at"].as_str().unwrap().ends_with('Z'));

    archive_user(&get_raw_pool(own_db_url().await).await, agent).await;
    let (status, body) = search!(app, caller, format!("?q={marker}"));
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        items(&body).is_empty(),
        "an archived agent is never found: {body}"
    );
}

#[tokio::test]
#[serial(process_env)]
async fn mair_1228_names_starting_with_the_term_come_first_and_limit_cuts() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let pool = get_pool().await;
    let app = init_app!(state);
    let marker = unique_marker("order");
    let caller = create_user(&pool, "Caller", &unique_marker("caller")).await;
    // « <marker>a Other » starts with the term; the two others only contain it (« x<marker> »),
    // and are newer: the prefix match still comes first.
    let starting = create_user(&pool, &format!("{marker}a"), "Other").await;
    create_user(&pool, "Zed", &format!("x{marker}")).await;
    create_user(&pool, "Third", &format!("x{marker}")).await;

    let (status, body) = search!(app, caller, format!("?q={marker}&limit=2"));
    assert_eq!(status, StatusCode::OK, "{body}");
    let found = items(&body);
    assert_eq!(found.len(), 2, "limit cuts the list: {body}");
    assert_eq!(
        found[0]["id"], starting,
        "a name starting with the term first: {body}"
    );
}

#[tokio::test]
#[serial(process_env)]
async fn mair_1228_rejects_a_short_term_a_bad_limit_and_no_session() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let pool = get_pool().await;
    let app = init_app!(state);
    let caller = create_user(&pool, "Caller", &unique_marker("caller")).await;

    for query in [
        "?q=a",
        "?q=%20a%20",
        "",
        "?q=ab&limit=21",
        "?q=ab&limit=0",
        "?q=ab&limit=abc",
        "?q=ab&limit=-1",
    ] {
        let (status, body) = search!(app, caller, query);
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {body}");
    }
    let anonymous = test::TestRequest::get()
        .uri("/api/v1/search?q=dupont")
        .to_request();
    let status = match test::try_call_service(&app, anonymous).await {
        Ok(resp) => resp.status(),
        Err(error) => error.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial(process_env)]
async fn mair_1228_like_wildcards_match_themselves() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let pool = get_pool().await;
    let app = init_app!(state);
    let caller = create_user(&pool, "Caller", &unique_marker("caller")).await;
    let marker = unique_marker("wildcard");
    create_user(&pool, "Agent", &marker).await;

    // Unescaped, `%_<marker>` would match « Agent <marker> » (any character before the marker).
    let (status, body) = search!(app, caller, format!("?q=%25_{marker}"));
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(items(&body).is_empty(), "{body}");
}
