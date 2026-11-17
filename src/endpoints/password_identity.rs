//! The identity rule of the password policy (MAIR-757).
//!
//! On the routes that set the password of an existing account, the password must not contain the account's e-mail (local part), first name
//! or last name. The length and common-password rules are applied by the views' `Validate`.

use crate::database::users::get_user_by_id::{GetUserByIdQueryResultView, GetUserByIdQueryView};
use crate::endpoints::validation::{check_password_identity, ValidationError};
use crate::password_policy::Identity;
use mairie360_api_lib::smart_db::SmartDatabase;

/// Why the identity rule could not accept the password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityCheckError {
    /// The password contains part of the identity: the `400` to answer.
    Rejected(ValidationError),
    /// No account has this id.
    UnknownUser,
    /// The account could not be read.
    Database,
}

/// Values a request changes along with the password (an administrator renaming an account and
/// setting its password at once): the rule applies to the identity the account will have.
#[derive(Debug, Default, Clone, Copy)]
pub struct IdentityChanges<'a> {
    pub email: Option<&'a str>,
    pub first_name: Option<&'a str>,
    pub last_name: Option<&'a str>,
}

/// Checks `password` against the identity of account `user_id`, after `changes`.
///
/// # Errors
///
/// [`IdentityCheckError::Rejected`] naming `field` when the password contains part of the
/// identity, [`IdentityCheckError::UnknownUser`] when the account does not exist,
/// [`IdentityCheckError::Database`] when it cannot be read (logged).
pub async fn check(
    smart_db: &SmartDatabase,
    user_id: u64,
    field: &str,
    password: &str,
    changes: IdentityChanges<'_>,
) -> Result<(), IdentityCheckError> {
    let user: GetUserByIdQueryResultView = smart_db
        .fetch_all::<GetUserByIdQueryResultView, _>(&GetUserByIdQueryView::new(user_id))
        .await
        .map_err(|e| {
            tracing::error!("Password policy: could not read the account: {e}");
            IdentityCheckError::Database
        })?
        .into_iter()
        .next()
        .ok_or(IdentityCheckError::UnknownUser)?;
    let identity = Identity {
        email: changes.email.unwrap_or_else(|| user.email()),
        first_name: changes.first_name.unwrap_or_else(|| user.first_name()),
        last_name: changes.last_name.unwrap_or_else(|| user.last_name()),
    };
    check_password_identity(field, password, identity).map_err(IdentityCheckError::Rejected)
}
