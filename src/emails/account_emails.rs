use mairie360_api_lib::email::{MailerError, ResendMailer, TemplateMailer};
use mairie360_api_lib::env_manager::get_env_var;

use super::{
    AccountInvitationEmail, EmailChangeConfirmationEmail, EmailChangedEmail, LoginAlertEmail,
    PasskeyChange, PasskeyChangedEmail, PasswordResetEmail,
};
use crate::redis_keys::{
    EMAIL_CHANGE_TTL_SECONDS, FORGOT_PASSWORD_TTL_SECONDS, INVITATION_TTL_SECONDS,
};

/// Public URL of the login front, where the links of the e-mails lead
/// (`https://login.mairie360-eip.fr`).
pub const PUBLIC_URL: &str = "PUBLIC_URL";
/// Optional name of the town hall shown in the e-mails.
pub const MAIRIE_NAME: &str = "MAIRIE_NAME";
/// Optional public URL of the administration front, where the administrators' alerts link to
/// (`https://admin.mairie360-eip.fr`, MAIR-773).
pub const ADMIN_PUBLIC_URL: &str = "ADMIN_PUBLIC_URL";

/// The e-mails Core sends about an account, registered as app data by `main`.
#[derive(Clone)]
pub struct AccountEmails {
    mailer: ResendMailer,
    public_url: String,
    admin_url: Option<String>,
    mairie_name: Option<String>,
}

/// An absolute `http(s)` URL without its trailing slash, `None` otherwise.
fn absolute_url(url: &str) -> Option<String> {
    let url = url.trim().trim_end_matches('/');
    let has_host = url
        .split_once("://")
        .is_some_and(|(scheme, rest)| (scheme == "https" || scheme == "http") && !rest.is_empty());
    (has_host && !url.chars().any(char::is_whitespace)).then(|| url.to_string())
}

impl AccountEmails {
    /// # Errors
    ///
    /// [`MailerError::Config`] when `public_url` is not an absolute `http(s)` URL.
    pub fn new(
        mailer: ResendMailer,
        public_url: &str,
        mairie_name: Option<String>,
    ) -> Result<Self, MailerError> {
        let Some(public_url) = absolute_url(public_url) else {
            return Err(MailerError::Config(format!(
                "{PUBLIC_URL} must be an absolute http(s) URL, got {public_url:?}"
            )));
        };
        Ok(Self {
            mailer,
            public_url,
            admin_url: None,
            mairie_name: mairie_name
                .map(|name| name.trim().to_string())
                .filter(|name| !name.is_empty()),
        })
    }

    /// Sets the URL of the administration front the administrators' alerts link to (MAIR-773). An
    /// invalid URL is ignored with a warning: the alerts are then sent without the link.
    #[must_use]
    pub fn with_admin_url(mut self, admin_url: Option<&str>) -> Self {
        self.admin_url = admin_url.and_then(|url| {
            let parsed = absolute_url(url);
            if parsed.is_none() {
                tracing::warn!("{ADMIN_PUBLIC_URL} is not an absolute http(s) URL: ignored");
            }
            parsed
        });
        self
    }

    /// The link of the stolen-session alert of an agent: `<PUBLIC_URL>/forgot-password`, where the
    /// agent asks for a password reset link (MAIR-773).
    #[must_use]
    pub fn change_password_url(&self) -> String {
        format!("{}/forgot-password", self.public_url)
    }

    /// The link of the administrators' alert: the sign-in journal of the agent in the administration,
    /// `<ADMIN_PUBLIC_URL>/?tab=security&journal=<user id>`; `None` without `ADMIN_PUBLIC_URL`.
    #[must_use]
    pub fn journal_url(&self, user_id: i64) -> Option<String> {
        self.admin_url
            .as_ref()
            .map(|admin_url| format!("{admin_url}/?tab=security&journal={user_id}"))
    }

    /// Reads `RESEND_API_KEY`, `EMAIL_DOMAIN`, `EMAIL_SENDER_NAME` (optional), `PUBLIC_URL`,
    /// `ADMIN_PUBLIC_URL` (optional) and `MAIRIE_NAME` (optional).
    ///
    /// # Errors
    ///
    /// [`MailerError::Config`] when a required variable is missing or invalid.
    pub fn from_env() -> Result<Self, MailerError> {
        let public_url = get_env_var(PUBLIC_URL)
            .ok_or_else(|| MailerError::Config(format!("{PUBLIC_URL} is not set")))?;
        Ok(Self::new(
            ResendMailer::from_env()?,
            &public_url,
            get_env_var(MAIRIE_NAME),
        )?
        .with_admin_url(get_env_var(ADMIN_PUBLIC_URL).as_deref()))
    }

    /// The link of the password reset e-mail: `<PUBLIC_URL>/reset-password?token=<token>`.
    #[must_use]
    pub fn reset_url(&self, token: &str) -> String {
        format!("{}/reset-password?token={token}", self.public_url)
    }

    /// The link of the e-mail change confirmation: `<PUBLIC_URL>/confirm-email?token=<token>`.
    #[must_use]
    pub fn confirm_email_url(&self, token: &str) -> String {
        format!("{}/confirm-email?token={token}", self.public_url)
    }

    /// Sends the confirmation link of an e-mail change to the NEW address.
    ///
    /// # Errors
    ///
    /// The [`MailerError`] of Resend.
    pub async fn send_email_change_confirmation(
        &self,
        new_email: &str,
        first_name: &str,
        token: &str,
    ) -> Result<(), MailerError> {
        let email = EmailChangeConfirmationEmail {
            first_name: first_name.to_string(),
            confirm_url: self.confirm_email_url(token),
            new_email: new_email.to_string(),
            expire_minutes: EMAIL_CHANGE_TTL_SECONDS / 60,
            mairie_name: self.mairie_name.clone(),
        };
        self.mailer.send(new_email, &email).await.map(|_| ())
    }

    /// Tells the OLD address that the account now uses `new_email`.
    ///
    /// # Errors
    ///
    /// The [`MailerError`] of Resend.
    pub async fn send_email_changed(
        &self,
        old_email: &str,
        first_name: &str,
        new_email: &str,
    ) -> Result<(), MailerError> {
        let email = EmailChangedEmail {
            first_name: first_name.to_string(),
            new_email: new_email.to_string(),
            date: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            mairie_name: self.mairie_name.clone(),
        };
        self.mailer.send(old_email, &email).await.map(|_| ())
    }

    /// Sends the password reset e-mail to `to`.
    ///
    /// # Errors
    ///
    /// The [`MailerError`] of Resend.
    pub async fn send_password_reset(
        &self,
        to: &str,
        first_name: &str,
        token: &str,
    ) -> Result<(), MailerError> {
        let email = PasswordResetEmail {
            first_name: first_name.to_string(),
            reset_url: self.reset_url(token),
            expire_minutes: FORGOT_PASSWORD_TTL_SECONDS / 60,
            mairie_name: self.mairie_name.clone(),
        };
        self.mailer.send(to, &email).await.map(|_| ())
    }

    /// Sends the invitation e-mail of an account an administrator created: a link to the password
    /// page of the login front, valid [`INVITATION_TTL_SECONDS`].
    ///
    /// # Errors
    ///
    /// The [`MailerError`] of Resend.
    pub async fn send_account_invitation(
        &self,
        to: &str,
        first_name: &str,
        token: &str,
    ) -> Result<(), MailerError> {
        let email = AccountInvitationEmail {
            first_name: first_name.to_string(),
            set_password_url: self.reset_url(token),
            expire_days: INVITATION_TTL_SECONDS / (24 * 60 * 60),
            mairie_name: self.mairie_name.clone(),
        };
        self.mailer.send(to, &email).await.map(|_| ())
    }

    /// Sends the security e-mail about a passkey added or removed (MAIR-810).
    ///
    /// # Errors
    ///
    /// The [`MailerError`] of Resend.
    pub async fn send_passkey_changed(
        &self,
        to: &str,
        first_name: &str,
        change: PasskeyChange,
        label: &str,
        device: &str,
        ip_address: &str,
    ) -> Result<(), MailerError> {
        let email = PasskeyChangedEmail {
            first_name: first_name.to_string(),
            change,
            label: label.to_string(),
            device: device.to_string(),
            date: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            ip_address: ip_address.to_string(),
            mairie_name: self.mairie_name.clone(),
        };
        self.mailer.send(to, &email).await.map(|_| ())
    }

    /// Sends the sign-in alert of an account that turned it on (MAIR-823).
    ///
    /// # Errors
    ///
    /// The [`MailerError`] of Resend.
    pub async fn send_login_alert(
        &self,
        to: &str,
        first_name: &str,
        device: &str,
        ip_address: &str,
    ) -> Result<(), MailerError> {
        let email = LoginAlertEmail {
            first_name: first_name.to_string(),
            device: device.to_string(),
            ip_address: ip_address.to_string(),
            date: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            mairie_name: self.mairie_name.clone(),
        };
        self.mailer.send(to, &email).await.map(|_| ())
    }

    /// Sends the stolen-session alert to the agent (MAIR-773, `account-session-alert`).
    ///
    /// # Errors
    ///
    /// The [`MailerError`] of Resend.
    pub async fn send_session_alert(
        &self,
        to: &str,
        first_name: &str,
        level: &str,
        device: &str,
        ip_address: &str,
        date: &str,
    ) -> Result<(), MailerError> {
        let email = super::SessionAlertEmail {
            first_name: first_name.to_string(),
            certainty: super::certainty_label(level).to_string(),
            device: device.to_string(),
            ip_address: super::mask_ip_address(ip_address),
            date: date.to_string(),
            change_password_url: self.change_password_url(),
            mairie_name: self.mairie_name.clone(),
        };
        self.mailer.send(to, &email).await.map(|_| ())
    }

    /// Sends the stolen-session alert of `agent_name` to an administrator (MAIR-773,
    /// `admin-session-alert`).
    ///
    /// # Errors
    ///
    /// The [`MailerError`] of Resend.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_admin_session_alert(
        &self,
        to: &str,
        agent_id: i64,
        agent_name: &str,
        level: &str,
        device: &str,
        ip_address: &str,
        date: &str,
    ) -> Result<(), MailerError> {
        let email = super::AdminSessionAlertEmail {
            agent_name: agent_name.to_string(),
            certainty: super::certainty_label(level).to_string(),
            device: device.to_string(),
            ip_address: super::mask_ip_address(ip_address),
            date: date.to_string(),
            journal_url: self.journal_url(agent_id),
            mairie_name: self.mairie_name.clone(),
        };
        self.mailer.send(to, &email).await.map(|_| ())
    }
}
