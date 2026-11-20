//! Invitation of the accounts that have no password (the accounts of the former Keycloak
//! synchronisation): each receives the « Définir votre mot de passe » e-mail, whose one-time link
//! is valid [`crate::redis_keys::INVITATION_TTL_SECONDS`]. Run before an instance switches, by the
//! `invite_passwordless` binary.

use crate::database::users::list_without_password::{
    ListUsersWithoutPasswordQueryView, UserWithoutPassword,
};
use crate::emails::{invite_account, AccountEmails};
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;
use serde::Serialize;

/// What a run did. It holds ids only: a job log must not carry e-mail addresses.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct InvitationReport {
    /// Accounts without a password.
    pub total: usize,
    /// Invitations sent (always `0` in a dry run).
    pub sent: usize,
    /// Ids of the accounts whose invitation failed, to replay.
    pub failed_ids: Vec<i64>,
    /// `true` when nothing was sent.
    pub dry_run: bool,
}

/// The accounts without a password.
///
/// # Errors
///
/// The database error, as text.
pub async fn pending_accounts(db: &SmartDatabase) -> Result<Vec<UserWithoutPassword>, String> {
    db.fetch_all::<UserWithoutPassword, _>(&ListUsersWithoutPasswordQueryView::new())
        .await
        .map_err(|e| e.to_string())
}

/// Sends the invitation to every account without a password, or only counts them when `dry_run`.
/// A failed e-mail is recorded and the run goes on; replaying invites everyone still without a
/// password, each with a fresh link.
///
/// # Errors
///
/// The database error when the accounts cannot be listed.
pub async fn invite_pending(
    state: &AppState,
    emails: &AccountEmails,
    dry_run: bool,
) -> Result<InvitationReport, String> {
    let pending = pending_accounts(state.get_smart_db()).await?;
    let mut report = InvitationReport {
        total: pending.len(),
        dry_run,
        ..InvitationReport::default()
    };
    if dry_run {
        return Ok(report);
    }
    for user in pending {
        match invite_account(state.get_redis(), emails, &user.email, &user.first_name).await {
            Ok(()) => report.sent += 1,
            Err(e) => {
                tracing::error!("Invitation of user {}: {e}", user.id);
                report.failed_ids.push(user.id);
            }
        }
    }
    Ok(report)
}
