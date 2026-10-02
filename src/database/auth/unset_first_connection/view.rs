use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Sets the password of `user_id` and completes its first connection. Run with `fetch_scalar`,
/// it returns the number of accounts changed (`i64`, 0 or 1).
///
/// [`Self::pending_only`] only changes an account whose first connection is still pending, in the
/// same statement as the check (MAIR-420): two concurrent requests cannot both pass it.
#[derive(serde::Deserialize)]
pub struct UnsetFirstConnectionQueryView {
    user_id: u64,
    password: String,
    pending_only: bool,
    params: Vec<QueryParam>,
}

impl UnsetFirstConnectionQueryView {
    #[must_use]
    pub fn new(user_id: u64, password: &str) -> Self {
        Self {
            user_id,
            password: password.to_string(),
            pending_only: false,
            params: vec![
                QueryParam::Text(password.to_string()),
                QueryParam::I32(user_id as i32),
            ],
        }
    }

    /// Same, but only for an active account whose first connection is still pending.
    #[must_use]
    pub fn pending_only(user_id: u64, password: &str) -> Self {
        Self {
            pending_only: true,
            ..Self::new(user_id, password)
        }
    }

    #[must_use]
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }

    #[must_use]
    pub fn password(&self) -> &str {
        &self.password
    }
}

impl ApiRequestDto for UnsetFirstConnectionQueryView {
    fn query_sql(&self) -> &'static str {
        if self.pending_only {
            "WITH changed AS (\
                UPDATE users SET first_connect = false, password = $1 \
                WHERE id = $2 AND first_connect = true AND COALESCE(is_archived, false) = false \
                RETURNING id\
            ) SELECT COUNT(*) FROM changed"
        } else {
            "WITH changed AS (\
                UPDATE users SET first_connect = false, password = $1 WHERE id = $2 RETURNING id\
            ) SELECT COUNT(*) FROM changed"
        }
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for UnsetFirstConnectionQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UnsetFirstConnectionQueryView: user_id = {}, password = [PROTECTED]",
            self.user_id
        )
    }
}
