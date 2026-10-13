//! E-mails sent by Core (MAIR-522).
//!
//! Every e-mail is a published Resend template that Core fills in (`mairie360_api_lib::email`):
//! no body is written here, and the sender is always `no-reply@<EMAIL_DOMAIN>`.

mod account_emails;
mod password_reset;

pub use account_emails::{AccountEmails, PUBLIC_URL};
pub use password_reset::PasswordResetEmail;
