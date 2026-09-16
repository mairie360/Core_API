use utoipa::ToSchema;

/// Ajout d'un utilisateur à un groupe.
#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PostUserGroupView {
    /// Identifiant de l'utilisateur à rattacher.
    #[schema(example = 42)]
    user_id: u64,
    /// Identifiant du groupe de destination. C'est cette valeur qui fait foi, pas le `group_id`
    /// du chemin.
    #[schema(example = 3)]
    group_id: u64,
}

impl PostUserGroupView {
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }

    pub const fn group_id(&self) -> u64 {
        self.group_id
    }
}
