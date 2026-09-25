//! Who may manage the accesses (ACL entries) of a resource instance.
//!
//! An administrator (same `is_admin()` rule as `AdminMiddleware` on `/admin`) may manage any
//! instance. Anybody else must own the instance: its `owner_id` equals the caller's id. Only the
//! resource types listed in `IsOwnerQueryView` have an owner; the others are admin-only.

use crate::database::ressources::is_owner::IsOwnerQueryView;
use mairie360_api_lib::database::query_views::IsAdminQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

/// # Errors
///
/// Returns an [`ApiLibError`] when the database cannot be queried.
pub async fn is_admin(smart_db: &SmartDatabase, caller_id: u64) -> Result<bool, ApiLibError> {
    smart_db
        .fetch_scalar::<bool, _>(&IsAdminQueryView::new(caller_id))
        .await
}

/// Whether `caller_id` owns the instance `instance_id` of `ressource_type`.
///
/// # Errors
///
/// Returns an [`ApiLibError`] when the database cannot be queried.
pub async fn is_owner(
    smart_db: &SmartDatabase,
    caller_id: u64,
    ressource_type: &str,
    instance_id: u64,
) -> Result<bool, ApiLibError> {
    if !IsOwnerQueryView::supports(ressource_type) {
        return Ok(false);
    }
    smart_db
        .fetch_scalar::<bool, _>(&IsOwnerQueryView::new(
            caller_id,
            instance_id,
            ressource_type,
        ))
        .await
}

/// Whether `caller_id` may add, remove or list the accesses of the instance `instance_id` of
/// `ressource_type`: an administrator, or the owner of the instance.
///
/// # Errors
///
/// Returns an [`ApiLibError`] when the database cannot be queried.
pub async fn can_manage_accesses(
    smart_db: &SmartDatabase,
    caller_id: u64,
    ressource_type: &str,
    instance_id: u64,
) -> Result<bool, ApiLibError> {
    if is_admin(smart_db, caller_id).await? {
        return Ok(true);
    }
    is_owner(smart_db, caller_id, ressource_type, instance_id).await
}

/// Whether `ressource_type` can be looked up in the `resources` table (`name VARCHAR(64)`):
/// 1 to 64 characters, none of them a control character (Postgres rejects NUL bytes).
#[must_use]
pub fn is_valid_ressource_type(ressource_type: &str) -> bool {
    !ressource_type.is_empty()
        && ressource_type.chars().count() <= 64
        && !ressource_type.chars().any(char::is_control)
}

/// Whether `id` fits the `INT` columns of `access_control`.
#[must_use]
pub fn fits_int_column(id: u64) -> bool {
    i32::try_from(id).is_ok()
}
