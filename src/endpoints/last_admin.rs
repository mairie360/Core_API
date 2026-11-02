//! Last-administrator guard (MAIR-678).
//!
//! Since a user holds one role (MAIR-564), granting a role to an administrator demotes them, and
//! nothing stopped an administrator from demoting, revoking or archiving the last active one: a
//! single call left the instance without any administration access. The role grant, the role
//! revocation and the archiving now run their write in a transaction opened by [`lock`] and
//! refuse with `409` ([`LAST_ADMIN_MESSAGE`]) when [`any_left`] finds no active administrator
//! after it, which rolls the write back.
//!
//! Lock choice: the transaction first locks the `Admin` role row (`FOR NO KEY UPDATE`, see
//! [`LockAdminRoleQueryView`]). Every guarded write takes that same row lock, so two demotions of
//! the two last administrators run one after the other and the second one sees the first
//! committed. Locking the administrators' `user_roles` rows would not be enough (a concurrent
//! write may target a row the other did not lock), and an advisory lock would add a magic key
//! next to the row locks every other check of Core already uses (MAIR-420).

use crate::database::roles::last_admin::{
    HasActiveAdminQueryView, IsLastActiveAdminQueryView, LockAdminRoleQueryView,
};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::{SmartDatabase, SmartTransaction};

/// Body of the `409` answered when a write would leave no active administrator.
pub const LAST_ADMIN_MESSAGE: &str = "This user is the last active administrator: give the Admin \
                                      role to another active user first.";

/// Name of the administrator role.
pub const ADMIN_ROLE_NAME: &str = "Admin";

/// Locks the `Admin` role row until the end of `tx`. Run it before the guarded write.
///
/// # Errors
///
/// Any database error.
pub async fn lock(tx: &mut SmartTransaction) -> Result<(), ApiLibError> {
    let _: Vec<i32> = tx.fetch_all(&LockAdminRoleQueryView::new()).await?;
    Ok(())
}

/// Whether an active administrator is left, as seen by `tx` (its own writes included). Run it
/// after the guarded write; `false` means the transaction must be rolled back.
///
/// # Errors
///
/// Any database error.
pub async fn any_left(tx: &mut SmartTransaction) -> Result<bool, ApiLibError> {
    tx.fetch_scalar(&HasActiveAdminQueryView::new()).await
}

/// Unlocked early check: whether `user_id` is the only active administrator. Used before a
/// change is mirrored into Keycloak, so that a change Core will refuse leaves the realm alone.
///
/// # Errors
///
/// Any database error.
pub async fn is_last(smart_db: &SmartDatabase, user_id: u64) -> Result<bool, ApiLibError> {
    smart_db
        .fetch_scalar(&IsLastActiveAdminQueryView::new(user_id))
        .await
}
