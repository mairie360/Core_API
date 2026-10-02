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
    /// Deprecated: the first of `roles` (lowest role id), empty string when the user has none.
    /// Read `roles` instead, a user can hold several roles.
    #[schema(example = "agent")]
    #[deprecated(note = "read `roles`")]
    role: String,
    /// Names of every role the user holds, ordered by role id. Empty when the user has none.
    #[schema(example = json!(["agent", "manager"]))]
    roles: Vec<String>,
    /// Groupes dont l'utilisateur est membre. Vide s'il n'appartient à aucun.
    groups: Vec<Group>,
}

impl GetUserResponseView {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    #[allow(deprecated)]
    pub fn new(
        first_name: &str,
        last_name: &str,
        email: &str,
        phone: Option<&str>,
        status: &str,
        is_archived: bool,
        roles: Vec<String>,
        groups: Vec<Group>,
    ) -> Self {
        Self {
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
            phone: phone.map(std::string::ToString::to_string),
            status: status.to_string(),
            is_archived,
            role: roles.first().cloned().unwrap_or_default(),
            roles,
            groups,
        }
    }

    #[must_use]
    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    #[must_use]
    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }

    #[must_use]
    pub fn phone(&self) -> Option<&str> {
        self.phone.as_deref()
    }

    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    #[must_use]
    pub const fn is_archived(&self) -> bool {
        self.is_archived
    }

    #[must_use]
    #[allow(deprecated)]
    pub fn role(&self) -> &str {
        &self.role
    }

    #[must_use]
    pub fn roles(&self) -> &[String] {
        &self.roles
    }
}

impl Display for GetUserResponseView {
    #[allow(deprecated)]
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
    #[allow(deprecated)]
    fn from(query_result: GetUserByIdQueryResultView) -> Self {
        Self {
            first_name: query_result.first_name().to_string(),
            last_name: query_result.last_name().to_string(),
            email: query_result.email().to_string(),
            phone: query_result
                .phone_number()
                .map(std::string::ToString::to_string),
            status: query_result.status().to_string(),
            is_archived: query_result.is_archived(),
            role: String::new(),
            roles: Vec::new(),
            groups: vec![],
        }
    }
}
