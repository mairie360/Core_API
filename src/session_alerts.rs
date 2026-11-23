//! Delivery of the stolen-session alerts (MAIR-773).
//!
//! Database queues one alert per session and level (`report_suspicious_session_use()`, called by
//! `POST /api/v1/sessions/report`). Core e-mails each alert to the agent (`account-session-alert`) and to
//! every active administrator (`admin-session-alert`), right after the report and from a sweep every
//! minute, which retries an alert that could not be sent (5 attempts, one per minute). Each replica claims
//! its alerts with `SKIP LOCKED`, so an alert is never sent twice at once. A failure is recorded in
//! `last_error` (its kind, never a value) and logged.

use crate::database::security_alerts::{
    ActiveAdminEmailsQueryView, ClaimUnsentAlertsQueryView, ClaimedAlert,
    RecordAlertDeliveryQueryView,
};
use crate::emails::AccountEmails;
use actix_web::web;
use mairie360_api_lib::state::AppState;
use std::time::Duration;

/// Interval of the sweep.
pub const SWEEP_INTERVAL: Duration = Duration::from_secs(60);

/// Sends every alert this replica can claim. Returns how many were delivered.
pub async fn deliver_pending(state: &AppState, emails: &AccountEmails) -> usize {
    let smart_db = state.get_smart_db();
    let alerts: Vec<ClaimedAlert> =
        match smart_db.fetch_all(&ClaimUnsentAlertsQueryView::new()).await {
            Ok(alerts) => alerts,
            Err(e) => {
                tracing::error!("Session alerts: claim failed: {e}");
                return 0;
            }
        };
    if alerts.is_empty() {
        return 0;
    }
    let admins: Vec<String> = match smart_db.fetch_all(&ActiveAdminEmailsQueryView::new()).await {
        Ok(admins) => admins,
        Err(e) => {
            tracing::error!("Session alerts: administrators unreadable: {e}");
            Vec::new()
        }
    };
    let mut delivered = 0;
    for alert in &alerts {
        let outcome = deliver(emails, alert, &admins).await;
        let record = match outcome {
            Ok(()) => {
                delivered += 1;
                RecordAlertDeliveryQueryView::sent(alert.id)
            }
            Err(reason) => {
                tracing::error!(
                    alert_id = alert.id,
                    user_id = alert.user_id,
                    "Session alert not sent: {reason}"
                );
                RecordAlertDeliveryQueryView::failed(alert.id, reason)
            }
        };
        if let Err(e) = smart_db.execute(record).await {
            tracing::error!(
                alert_id = alert.id,
                "Session alert: delivery not recorded: {e}"
            );
        }
    }
    delivered
}

/// Sends one alert: the agent's e-mail decides; the administrators' are best effort.
async fn deliver(
    emails: &AccountEmails,
    alert: &ClaimedAlert,
    admins: &[String],
) -> Result<(), &'static str> {
    let device = alert.device_info.as_deref().unwrap_or("");
    let ip_address = alert.ip_address.as_deref().unwrap_or("");
    emails
        .send_session_alert(
            &alert.email,
            &alert.first_name,
            &alert.level,
            device,
            ip_address,
            &alert.created_at,
        )
        .await
        .map_err(|_| "the agent's e-mail was refused")?;
    let agent_name = format!("{} {}", alert.first_name, alert.last_name);
    for admin in admins {
        if let Err(e) = emails
            .send_admin_session_alert(
                admin,
                &agent_name,
                &alert.level,
                device,
                ip_address,
                &alert.created_at,
            )
            .await
        {
            tracing::error!(
                alert_id = alert.id,
                "Session alert: an administrator's e-mail was refused: {e}"
            );
        }
    }
    Ok(())
}

/// Delivers the pending alerts on a background task (after a report).
pub fn spawn_delivery(state: &web::Data<AppState>, emails: Option<&web::Data<AccountEmails>>) {
    let Some(emails) = emails.cloned() else {
        tracing::error!("Session alerts: no mailer is registered, the alert waits for the sweep");
        return;
    };
    let state = state.clone();
    tokio::spawn(async move {
        deliver_pending(&state, &emails).await;
    });
}

/// Starts the sweep of `main`: every [`SWEEP_INTERVAL`], the alerts not sent yet are retried.
pub fn spawn_sweeper(state: web::Data<AppState>, emails: web::Data<AccountEmails>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(SWEEP_INTERVAL);
        loop {
            interval.tick().await;
            deliver_pending(&state, &emails).await;
        }
    });
}
