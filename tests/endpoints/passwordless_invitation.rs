//! Accounts without a password (the ones the former Keycloak synchronisation created): the
//! `invite_passwordless` job lists them and e-mails each the « Définir votre mot de passe » link.

use crate::common::fake_resend::{self, FailingGuard};
use crate::common::get_raw_pool;
use crate::common::users::{archive_user, create_user, unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::invitations::{invite_pending, pending_accounts};
use mairie360_api_lib::security::JwtMiddleware;
use mairie360_api_lib::state::AppState;
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use serde_json::json;
use serial_test::serial;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as the other endpoint tests: they share the test binary.
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

async fn email_of(host: &str, id: i32) -> String {
    sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&get_raw_pool(host.to_string()).await)
        .await
        .unwrap()
}

async fn clear_password(host: &str, id: i32) {
    sqlx::query("UPDATE users SET password = NULL WHERE id = $1")
        .bind(id)
        .execute(&get_raw_pool(host.to_string()).await)
        .await
        .unwrap();
}

/// Only a non-archived account without a password is listed, a dry run sends nothing, and the
/// real run e-mails exactly those accounts.
#[tokio::test]
#[serial(fake_resend)]
async fn only_the_accounts_without_a_password_are_invited() {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let (_redis, redis) = start_redis_container().await;
    let state = AppState::new(redis.url.clone(), host.clone()).await;
    let emails = fake_resend::account_emails();
    let marker = unique_marker("nopass");

    let without = create_user(state.get_smart_db(), "Nora", &marker).await;
    let with = create_user(state.get_smart_db(), "Paul", &marker).await;
    let archived = create_user(state.get_smart_db(), "Alix", &marker).await;
    clear_password(&host, without).await;
    clear_password(&host, archived).await;
    archive_user(&get_raw_pool(host.clone()).await, archived).await;
    let (without_email, with_email, archived_email) = (
        email_of(&host, without).await,
        email_of(&host, with).await,
        email_of(&host, archived).await,
    );

    let listed: Vec<i64> = pending_accounts(state.get_smart_db())
        .await
        .unwrap()
        .iter()
        .map(|u| u.id)
        .collect();
    assert!(listed.contains(&i64::from(without)), "{listed:?}");
    assert!(!listed.contains(&i64::from(with)), "has a password");
    assert!(!listed.contains(&i64::from(archived)), "archived");

    let dry = invite_pending(&state, &emails, true).await.unwrap();
    assert!(dry.dry_run && dry.sent == 0 && dry.total == listed.len());
    assert!(fake_resend::sent_to(&without_email).is_empty(), "dry run");

    let report = invite_pending(&state, &emails, false).await.unwrap();
    assert_eq!(report.sent, report.total, "{report:?}");
    assert!(report.failed_ids.is_empty());
    let sent = fake_resend::sent_to(&without_email);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0]["template"]["id"], "account-invitation");
    assert_eq!(sent[0]["template"]["variables"]["USER_FIRST_NAME"], "Nora");
    assert!(fake_resend::sent_to(&with_email).is_empty());
    assert!(fake_resend::sent_to(&archived_email).is_empty());
}

/// The business journey: the invitation link gives the account its first password, then it signs
/// in and the account is no longer listed.
#[tokio::test]
#[serial(fake_resend)]
async fn the_invited_account_sets_its_password_and_signs_in() {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), host.clone()).await);
    let emails = fake_resend::account_emails();
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .app_data(emails.clone())
            .configure(public_config)
            .service(
                web::scope("/api")
                    .wrap(actix_web::middleware::from_fn(session_guard))
                    .wrap(JwtMiddleware)
                    .configure(config),
            ),
    )
    .await;
    let id = create_user(state.get_smart_db(), "Lena", &unique_marker("journey")).await;
    clear_password(&host, id).await;
    let email = email_of(&host, id).await;

    invite_pending(&state, &emails, false).await.unwrap();
    let sent = fake_resend::sent_to(&email);
    assert_eq!(sent.len(), 1, "{sent:?}");
    let token = fake_resend::invitation_token(&sent[0]);

    let reset = test::TestRequest::post()
        .uri("/api/v1/auth/reset_password")
        .set_json(
            json!({ "token": token, "new_password": "Chosen!Pass456", "device_info": "test" }),
        )
        .to_request();
    assert_eq!(
        test::call_service(&app, reset).await.status(),
        StatusCode::OK
    );
    let login = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({ "email": email, "password": "Chosen!Pass456", "device_info": "test" }))
        .to_request();
    assert_eq!(
        test::call_service(&app, login).await.status(),
        StatusCode::OK
    );

    let listed = pending_accounts(state.get_smart_db()).await.unwrap();
    assert!(listed.iter().all(|u| u.id != i64::from(id)), "still listed");
}

/// A refused e-mail is reported by id (never the address), leaves no token behind, and the next
/// run invites the account.
#[tokio::test]
#[serial(fake_resend)]
async fn a_failed_invitation_is_reported_and_replayed() {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let (_redis, redis) = start_redis_container().await;
    let state = AppState::new(redis.url.clone(), host.clone()).await;
    let emails = fake_resend::account_emails();
    let id = create_user(state.get_smart_db(), "Remi", &unique_marker("replay")).await;
    clear_password(&host, id).await;
    let email = email_of(&host, id).await;

    {
        let _failing = FailingGuard::set();
        let report = invite_pending(&state, &emails, false).await.unwrap();
        assert!(report.failed_ids.contains(&i64::from(id)), "{report:?}");
        assert!(!serde_json::to_string(&report).unwrap().contains(&email));
    }
    assert!(fake_resend::sent_to(&email).is_empty());

    let report = invite_pending(&state, &emails, false).await.unwrap();
    assert!(report.failed_ids.is_empty(), "{report:?}");
    assert_eq!(fake_resend::sent_to(&email).len(), 1);
}
