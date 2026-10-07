/// SQL expression a user search is matched against: the lowered first name + last name, last
/// name + first name and e-mail joined by `chr(31)`, so a term never matches across two fields.
/// It must stay the expression of `idx_users_search_text_trgm` (Database 3.0.1): one `LIKE` on it
/// replaces five `ILIKE` joined by OR, which read the whole table (14 to 74 ms on 10 000 users,
/// 0.7 to 3.9 ms this way, MAIR-477). Expects the table aliased `u`.
#[macro_export]
macro_rules! user_search_text_sql {
    () => {
        "lower(u.first_name || ' ' || u.last_name || chr(31) \
            || u.last_name || ' ' || u.first_name || chr(31) || u.email)"
    };
}

pub mod add_role;
pub mod delete_user;
pub mod get_notification_settings;
pub mod get_preferences;
pub mod get_roles;
pub mod get_user_by_id;
pub mod list_directory;
pub mod nullable_patch;
pub mod patch_notification_settings;
pub mod patch_preferences;
pub mod patch_user;
pub mod remove_role;
