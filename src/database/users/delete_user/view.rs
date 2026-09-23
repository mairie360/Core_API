use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Archives a user (users are never hard-deleted).
///
/// `v_users_active` has an `INSTEAD OF DELETE` trigger that sets `is_archived = TRUE` /
/// `status = 'archived'`. The
/// `fn_check_can_delete_user` trigger aborts it with a `restrict_violation` (SQLSTATE `23001`) when
/// the user still owns groups, events or projects.
///
/// The trigger returns `NULL`, so the statement reports no affected row even on success: check
/// the user with [`IsUserActiveQueryView`] first.
#[derive(serde::Deserialize)]
pub struct DeleteUserQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl DeleteUserQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(user_id as i32)],
        }
    }

    #[must_use]
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for DeleteUserQueryView {
    fn query_sql(&self) -> &'static str {
        "DELETE FROM v_users_active WHERE id = $1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for DeleteUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeleteUserQueryView: user_id = {}", self.user_id)
    }
}

/// Whether `user_id` is an existing, non-archived user.
#[derive(serde::Deserialize)]
pub struct IsUserActiveQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl IsUserActiveQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(user_id as i32)],
        }
    }
}

impl ApiRequestDto for IsUserActiveQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT EXISTS(SELECT 1 FROM v_users_active WHERE id = $1)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for IsUserActiveQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IsUserActiveQueryView: user_id = {}", self.user_id)
    }
}
