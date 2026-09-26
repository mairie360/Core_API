//! MAIR-225: `forgot_password` must not reveal whether an e-mail matches an account.

use crate::common::{get_pool, users::create_user, users::unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::json;
use serial_test::serial;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

/// Minimal SMTP server accepting every message, so the "known e-mail" path really sends its mail.
async fn start_fake_smtp() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut lines = BufReader::new(read).lines();
                let _ = write.write_all(b"220 localhost ESMTP\r\n").await;
                let mut in_data = false;
                while let Ok(Some(line)) = lines.next_line().await {
                    let reply: &[u8] = if in_data {
                        if line != "." {
                            continue;
                        }
                        in_data = false;
                        b"250 queued\r\n"
                    } else {
                        match line.get(..4).map(str::to_ascii_uppercase).as_deref() {
                            Some("DATA") => {
                                in_data = true;
                                b"354 go ahead\r\n"
                            }
                            Some("QUIT") => {
                                let _ = write.write_all(b"221 bye\r\n").await;
                                break;
                            }
                            _ => b"250 ok\r\n",
                        }
                    };
                    let _ = write.write_all(reply).await;
                }
            });
        }
    });
    port
}

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

macro_rules! forgot {
    ($app:expr, $email:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v1/auth/forgot_password")
            .set_json(json!({ "email": $email }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        let status = resp.status();
        let body = test::read_body(resp).await;
        (status, body)
    }};
}

#[tokio::test]
#[serial]
async fn known_and_unknown_emails_get_the_same_answer() {
    let (_container, host) = get_shared_db().await;
    let (_redis, redis_config) = start_redis_container().await;
    let smtp_port = start_fake_smtp().await;
    std::env::set_var("SMTP_HOST", "localhost");
    std::env::set_var("SMTP_PORT", smtp_port.to_string());
    std::env::remove_var("SMTP_USERNAME");

    let state = web::Data::new(AppState::new(redis_config.url.clone(), host.clone()).await);
    let app = init_app!(state);

    let pool = get_pool(host.clone()).await;
    let marker = unique_marker("forgot");
    let user_id = create_user(&pool, "Known", &marker).await;
    let known_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&crate::common::get_raw_pool(host.clone()).await)
        .await
        .unwrap();
    let unknown_email = format!("nobody.{marker}@example.com");

    let unknown = forgot!(app, unknown_email);
    let known = forgot!(app, known_email);
    // A reset is now pending for the known address: a second request must look the same too.
    let pending = forgot!(app, known_email);

    assert_eq!(unknown.0, StatusCode::OK, "{:?}", unknown.1);
    assert_eq!(known, unknown, "known e-mail answered differently");
    assert_eq!(pending, unknown, "pending reset answered differently");

    // The known address really got a reset token.
    let token: Option<String> = state
        .get_redis()
        .secure_get(&format!("{known_email}/forgot_password_token"))
        .await
        .unwrap();
    assert!(token.is_some());
    let token: Option<String> = state
        .get_redis()
        .secure_get(&format!("{unknown_email}/forgot_password_token"))
        .await
        .unwrap();
    assert!(token.is_none());
}
