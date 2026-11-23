//! E-mails sent by Core (MAIR-522).
//!
//! Every e-mail is a published Resend template that Core fills in (`mairie360_api_lib::email`):
//! no body is written here, and the sender is always `no-reply@<EMAIL_DOMAIN>`.

mod account_emails;
mod account_invitation;
mod email_change_confirmation;
mod email_changed;
mod invitation;
mod login_alert;
mod passkey_changed;
mod password_reset;

pub use account_emails::{AccountEmails, PUBLIC_URL};
pub use account_invitation::AccountInvitationEmail;
pub use email_change_confirmation::EmailChangeConfirmationEmail;
pub use email_changed::EmailChangedEmail;
pub use invitation::{invite_account, InviteError};
pub use login_alert::LoginAlertEmail;
pub use passkey_changed::{PasskeyChange, PasskeyChangedEmail};
pub use password_reset::PasswordResetEmail;
