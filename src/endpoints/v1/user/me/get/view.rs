use crate::database::{
    groups::get_group::Group, users::get_user_by_id::GetUserByIdQueryResultView,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Profil de l'utilisateur connecté.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct GetMeResponseView {
    /// Prénom de l'utilisateur connecté.
    #[schema(example = "Jean")]
    first_name: String,
    /// Nom de famille de l'utilisateur connecté.
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
    /// Nom du rôle principal de l'utilisateur. Chaîne vide s'il n'en a pas.
    #[schema(example = "agent")]
    role: String,
    /// Groupes dont l'utilisateur est membre. Vide s'il n'appartient à aucun.
    groups: Vec<Group>,
}

impl GetMeResponseView {
    pub fn new(
        first_name: &str,
        last_name: &str,
        email: &str,
        phone: Option<&str>,
        status: &str,
        role: &str,
        groups: Vec<Group>,
    ) -> Self {
        GetMeResponseView {
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
            phone: phone.map(|p| p.to_string()),
            status: status.to_string(),
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

    pub fn role(&self) -> &str {
        &self.role
    }

    pub fn groups(&self) -> &[Group] {
        &self.groups
    }
}

impl Display for GetMeResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetMeResponseView {{ first_name: {}, last_name: {}, email: {}, phone: {:?}, status: {}, role: {}, groups: {} }}",
            self.first_name,
            self.last_name,
            self.email,
            self.phone,
            self.status,
            self.role,
            self.groups.len(),
        )
    }
}

impl From<GetUserByIdQueryResultView> for GetMeResponseView {
    fn from(query_result: GetUserByIdQueryResultView) -> Self {
        GetMeResponseView {
            first_name: query_result.first_name().to_string(),
            last_name: query_result.last_name().to_string(),
            email: query_result.email().to_string(),
            phone: query_result.phone_number().map(|p| p.to_string()),
            status: query_result.status().to_string(),
            role: "".to_string(),
            groups: Vec::new(),
        }
    }
}
