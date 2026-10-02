//! Who may read a group and its member list (MAIR-390).

use crate::database::groups::is_user_member::IsUserMemberQueryView;
use mairie360_api_lib::database::query_views::HasAccessQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

/// Whether `user_id` may read the existing group `group_id`: its members, and whoever
/// `check_access` grants `read` on it (global `read_all` such as administrators, the owner, or an
/// individual / group ACL).
///
/// # Errors
///
/// Returns an [`ApiLibError`] when the database cannot answer.
pub async fn can_read_group(
    smart_db: &SmartDatabase,
    user_id: u64,
    group_id: u64,
) -> Result<bool, ApiLibError> {
    let is_member: bool = smart_db
        .fetch_scalar(&IsUserMemberQueryView::new(group_id, user_id))
        .await?;
    if is_member {
        return Ok(true);
    }
    let access: i32 = smart_db
        .fetch_scalar(&HasAccessQueryView::new(
            user_id,
            "groups",
            "read",
            Some(group_id),
        ))
        .await?;
    Ok(access == 1)
}
