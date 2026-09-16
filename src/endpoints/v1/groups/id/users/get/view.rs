use utoipa::ToSchema;

/// Membres d'un groupe.
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct GetGroupUsersResultView {
    /// Identifiants des membres du groupe, à repasser à `GET /api/v1/user/?ids=…` pour obtenir
    /// leurs fiches. Vide si le groupe n'a aucun membre.
    #[schema(example = json!([1, 2, 5]))]
    users: Vec<u64>,
}

impl GetGroupUsersResultView {
    pub const fn new(users: Vec<u64>) -> Self {
        Self { users }
    }
}

impl From<Vec<i32>> for GetGroupUsersResultView {
    fn from(users: Vec<i32>) -> Self {
        Self::new(users.into_iter().map(|u| u as u64).collect())
    }
}
