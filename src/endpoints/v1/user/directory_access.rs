//! Who may read the directory (MAIR-288, access-matrix.yaml).
//!
//! Every agent reads the directory by default (`DIRECTORY_GUEST_ACCESS=all`). A mairie can close
//! it to the agents that only hold the Guest role with `DIRECTORY_GUEST_ACCESS=none`: they then
//! get `403` on `GET /api/v1/user/` and on another user's record, and still read their own.

use crate::database::roles::get_roles_by_id::{GetRolesByIdQueryView, Role};
use crate::database::users::get_roles::GetUserRolesQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

/// Name of the role whose holders may lose the directory.
pub const GUEST_ROLE: &str = "Guest";

/// `true` when `DIRECTORY_GUEST_ACCESS` (read on every call) is `none`.
#[must_use]
pub fn closed_to_guests() -> bool {
    std::env::var("DIRECTORY_GUEST_ACCESS")
        .is_ok_and(|value| value.trim().eq_ignore_ascii_case("none"))
}

/// `true` when the user holds no role other than Guest (an account without any role counts as a
/// Guest: the schema gives every new account the Guest role).
///
/// # Errors
///
/// The database errors.
pub async fn is_guest_only(smart_db: &SmartDatabase, user_id: u64) -> Result<bool, ApiLibError> {
    let ids: Vec<i32> = smart_db
        .fetch_all(&GetUserRolesQueryView::new(user_id))
        .await?;
    if ids.is_empty() {
        return Ok(true);
    }
    let roles: Vec<Role> = smart_db.fetch_all(&GetRolesByIdQueryView::new(ids)).await?;
    Ok(roles.iter().all(|role| role.name() == GUEST_ROLE))
}

/// Whether `user_id` may read the directory and the records of other users.
///
/// # Errors
///
/// The database errors.
pub async fn may_read_directory(
    smart_db: &SmartDatabase,
    user_id: u64,
) -> Result<bool, ApiLibError> {
    if !closed_to_guests() {
        return Ok(true);
    }
    Ok(!is_guest_only(smart_db, user_id).await?)
}
