use serde::Deserialize;
use std::fmt::Display;
use utoipa::ToSchema;

/// Attribution d'un rôle à un utilisateur.
#[derive(Deserialize, ToSchema)]
pub struct AddRoleToUserView {
    /// Identifiant du rôle à attribuer, tel que renvoyé par `GET /api/v1/admin/roles/`.
    #[schema(example = 2)]
    role_id: u64,
    /// Identifiant de l'utilisateur qui reçoit le rôle. C'est cette valeur qui fait foi, pas
    /// l'`userId` du chemin.
    #[schema(example = 42)]
    user_id: u64,
}

impl AddRoleToUserView {
    pub const fn role_id(&self) -> u64 {
        self.role_id
    }

    pub const fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl Display for AddRoleToUserView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AddRoleToUserView {{ role_id: {}, user_id: {} }}",
            self.role_id, self.user_id
        )
    }
}
