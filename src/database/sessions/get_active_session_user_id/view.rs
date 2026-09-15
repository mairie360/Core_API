use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Renvoie l'`user_id` de la session active associée à un refresh token.
///
/// Sert au refresh du JWT, qui doit fonctionner quand le JWT est expiré : le refresh token est
/// alors la seule preuve d'identité. Aucune ligne (token inconnu, révoqué ou expiré) donne
/// `DbError::NotFound`.
#[derive(serde::Deserialize)]
pub struct GetActiveSessionUserIdQueryView {
    token_hash: String,
    params: Vec<QueryParam>,
}

impl GetActiveSessionUserIdQueryView {
    #[must_use]
    pub fn new(token_hash: &str) -> Self {
        Self {
            token_hash: token_hash.to_string(),
            params: vec![QueryParam::Text(token_hash.to_string())],
        }
    }

    #[must_use]
    pub fn get_token_hash(&self) -> &str {
        &self.token_hash
    }
}

impl ApiRequestDto for GetActiveSessionUserIdQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT user_id FROM v_sessions WHERE token_hash = $1 AND is_active = true LIMIT 1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetActiveSessionUserIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetActiveSessionUserIdQueryView: token_hash = [PROTECTED]"
        )
    }
}
