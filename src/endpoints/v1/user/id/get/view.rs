use crate::database::{
    groups::get_group::Group, users::get_user_by_id::GetUserByIdQueryResultView,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Profil public d'un utilisateur.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct GetUserResponseView {
    /// Prénom de l'utilisateur.
    #[schema(example = "Jean")]
    first_name: String,
    /// Nom de famille de l'utilisateur.
    #[schema(example = "Dupont")]
    last_name: String,
    /// Adresse e-mail, unique sur la plateforme.
    #[schema(format = Email, example = "jean.dupont@mairie360.fr")]
    email: String,
    /// Numéro de téléphone, ou `null` si l'utilisateur n'en a pas renseigné.
    #[schema(example = "0612345678")]
    phone: Option<String>,
    /// Statut du compte tel qu'il est stocké en base.
    #[schema(example = "active")]
    status: String,
    /// `true` si le compte a été archivé : il apparaît encore ici mais plus dans
    /// `GET /api/v1/user/`, et ne peut plus se connecter.
    #[schema(example = false)]
    is_archived: bool,
    /// Nom du rôle principal de l'utilisateur. Chaîne vide s'il n'en a pas.
    #[schema(example = "agent")]
    role: String,
    /// Groupes dont l'utilisateur est membre. Vide s'il n'appartient à aucun.
    groups: Vec<Group>,
}

impl GetUserResponseView {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        first_name: &str,
        last_name: &str,
        email: &str,
        phone: Option<&str>,
        status: &str,
        is_archived: bool,
        role: &str,
        groups: Vec<Group>,
    ) -> Self {
        GetUserResponseView {
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
            phone: phone.map(|p| p.to_string()),
            status: status.to_string(),
            is_archived,
            role: role.to_string(),
            groups,
        }
    }

    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn phone(&self) -> Option<&str> {
        self.phone.as_deref()
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn is_archived(&self) -> bool {
        self.is_archived
    }

    pub fn role(&self) -> &str {
        &self.role
    }
}

impl Display for GetUserResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetUserResponseView {{ first_name: {}, last_name: {}, email: {}, phone: {:?}, status: {}, is_archived: {}, role: {} }}",
            self.first_name,
            self.last_name,
            self.email,
            self.phone,
            self.status,
            self.is_archived,
            self.role,
        )
    }
}

impl From<GetUserByIdQueryResultView> for GetUserResponseView {
    fn from(query_result: GetUserByIdQueryResultView) -> Self {
        GetUserResponseView {
            first_name: query_result.first_name().to_string(),
            last_name: query_result.last_name().to_string(),
            email: query_result.email().to_string(),
            phone: query_result.phone_number().map(|p| p.to_string()),
            status: query_result.status().to_string(),
            is_archived: query_result.is_archived(),
            role: "".to_string(),
            groups: vec![],
        }
    }
}
