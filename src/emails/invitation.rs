use mairie360_api_lib::redis::redis_interface::Redis;
use uuid::Uuid;

use super::AccountEmails;
use crate::redis_keys::{set_token, INVITATION_TTL_SECONDS};

/// Why [`invite_account`] could not invite an address.
#[derive(Debug)]
pub enum InviteError {
    Token(String),
    Mail(String),
}

impl std::fmt::Display for InviteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Token(e) => write!(f, "the invitation token could not be stored: {e}"),
            Self::Mail(e) => write!(f, "the invitation e-mail could not be sent: {e}"),
        }
    }
}

impl std::error::Error for InviteError {}

/// E-mails the invitation of an account: a one-time link to the password page of the login front,
/// valid [`INVITATION_TTL_SECONDS`], consumed by `POST /auth/reset_password` like a forgot-password
/// token. Only the `token -> e-mail` mapping is stored: a pending invitation must not silence a
/// later `POST /auth/forgot_password` of the same address.
///
/// An unsent invitation leaves no token behind.
///
/// # Errors
///
/// [`InviteError`] when the token cannot be stored or the e-mail cannot be sent.
pub async fn invite_account(
    redis: &Redis,
    emails: &AccountEmails,
    email: &str,
    first_name: &str,
) -> Result<(), InviteError> {
    let token = Uuid::new_v4().to_string();
    let email_key = format!("{token}/forgot_password_email");
    set_token(redis, &email_key, email, INVITATION_TTL_SECONDS)
        .await
        .map_err(|e| InviteError::Token(e.to_string()))?;
    if let Err(e) = emails
        .send_account_invitation(email, first_name, &token)
        .await
    {
        if let Err(e) = redis.secure_delete(&email_key).await {
            tracing::warn!("Invitation: could not drop an unsent invitation token: {e}");
        }
        return Err(InviteError::Mail(e.to_string()));
    }
    Ok(())
}
