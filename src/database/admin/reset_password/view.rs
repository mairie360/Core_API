use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;
use uuid::Uuid;

/// Result of [`AdminResetPasswordQueryView`] (`fetch_one`).
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AdminResetPasswordResult {
    updated: bool,
    revoked_sessions: Vec<Uuid>,
}

impl AdminResetPasswordResult {
    /// `false` when no user has this id.
    #[must_use]
    pub const fn updated(&self) -> bool {
        self.updated
    }

    /// Sessions of the user revoked by the reset, to publish to the revocation list.
    #[must_use]
    pub fn revoked_sessions(&self) -> &[Uuid] {
        &self.revoked_sessions
    }
}

/// Remplace le mot de passe d'un utilisateur, lève son éventuelle première connexion et révoque ses
/// sessions actives, en une seule requête. Renvoie `false` si l'utilisateur n'existe pas.
#[derive(serde::Deserialize)]
pub struct AdminResetPasswordQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl AdminResetPasswordQueryView {
    pub fn new(user_id: u64, new_password: &str) -> Self {
        Self {
            user_id,
            params: vec![
                QueryParam::Text(new_password.to_string()),
                QueryParam::I32(user_id as i32),
            ],
        }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for AdminResetPasswordQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH updated AS ( \
            UPDATE users SET password = $1, first_connect = false WHERE id = $2 RETURNING id \
         ), revoked AS ( \
            UPDATE sessions SET revoked_at = NOW() \
            WHERE user_id IN (SELECT id FROM updated) AND revoked_at IS NULL RETURNING id \
         ) \
         SELECT json_build_object( \
            'updated', EXISTS (SELECT 1 FROM updated), \
            'revoked_sessions', COALESCE((SELECT json_agg(id) FROM revoked), '[]'::json) \
         )"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for AdminResetPasswordQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AdminResetPasswordQueryView: user_id = {}, password = [PROTECTED]",
            self.user_id
        )
    }
}
