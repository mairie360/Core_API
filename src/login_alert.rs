//! Sign-in alerts (MAIR-823): an account whose `login_alerts` notification setting is on gets the
//! Resend template `account-login-alert` at every new session (password, second step, passkey,
//! password reset, mandatory enrolment).
//!
//! The e-mail never delays nor blocks the sign-in: it is sent on a background task once the session
//! is open, and every failure (settings unreadable, no mailer, Resend refusal) is only logged.

use crate::database::users::get_notification_settings::{
    GetNotificationSettingsQueryView, UserNotificationSettings,
};
use crate::database::users::get_user_by_id::{GetUserByIdQueryResultView, GetUserByIdQueryView};
use crate::emails::AccountEmails;
use actix_web::web;
use mairie360_api_lib::state::AppState;

/// Sends the alert when the account asked for it. Awaited by [`spawn`]; public for the tests.
pub async fn notify(
    state: &AppState,
    emails: Option<&AccountEmails>,
    user_id: u64,
    device: &str,
    ip_address: &str,
) {
    let smart_db = state.get_smart_db();
    let settings: UserNotificationSettings = match smart_db
        .fetch_one(&GetNotificationSettingsQueryView::new(user_id))
        .await
    {
        Ok(settings) => settings,
        Err(e) => {
            tracing::warn!(
                user_id,
                "Login alert: notification settings unreadable: {e}"
            );
            return;
        }
    };
    if settings.login_alerts != Some(true) {
        return;
    }
    let Some(emails) = emails else {
        tracing::error!(
            user_id,
            "Login alert: no mailer is registered, the alert is not sent"
        );
        return;
    };
    let user: GetUserByIdQueryResultView = match smart_db
        .fetch_one(&GetUserByIdQueryView::new(user_id))
        .await
    {
        Ok(user) => user,
        Err(e) => {
            tracing::warn!(user_id, "Login alert: account unreadable: {e}");
            return;
        }
    };
    if let Err(e) = emails
        .send_login_alert(user.email(), user.first_name(), device, ip_address)
        .await
    {
        tracing::error!(user_id, "Login alert: the e-mail could not be sent: {e}");
    }
}

/// Sends the alert in the background: the sign-in answers without waiting for Resend.
pub fn spawn(
    state: &web::Data<AppState>,
    emails: Option<&web::Data<AccountEmails>>,
    user_id: u64,
    device: &str,
    ip_address: std::net::IpAddr,
) {
    let state = state.clone();
    let emails = emails.cloned();
    let device = device.to_string();
    let ip_address = ip_address.to_string();
    // A task of the runtime (not `actix_web::rt::spawn`, which needs actix's `LocalSet`): the
    // handlers also run outside one, in the endpoint tests.
    tokio::spawn(async move {
        notify(
            &state,
            emails.as_ref().map(web::Data::get_ref),
            user_id,
            &device,
            &ip_address,
        )
        .await;
    });
}
