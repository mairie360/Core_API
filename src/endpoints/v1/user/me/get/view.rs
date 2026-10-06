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
    /// Phone number in E.164 (`+33612345678`), or `null` when the user has none. A legacy number
    /// that could not be attributed to a country (MAIR-480) is returned as stored, digits only,
    /// with `phone_country` `null`.
    #[schema(example = "+33612345678")]
    phone: Option<String>,
    /// ISO 3166-1 alpha-2 code of the phone number's country (`FR`), `null` when `phone` is.
    #[schema(example = "FR")]
    phone_country: Option<String>,
    /// Statut du compte tel qu'il est stocké en base.
    #[schema(example = "active")]
    status: String,
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

impl GetMeResponseView {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    #[allow(deprecated)]
    pub fn new(
        first_name: &str,
        last_name: &str,
        email: &str,
        phone: Option<String>,
        phone_country: Option<&str>,
        status: &str,
        roles: Vec<String>,
        groups: Vec<Group>,
    ) -> Self {
        Self {
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
            phone,
            phone_country: phone_country.map(std::string::ToString::to_string),
            status: status.to_string(),
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
    pub fn phone_country(&self) -> Option<&str> {
        self.phone_country.as_deref()
    }

    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
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

    #[must_use]
    pub fn groups(&self) -> &[Group] {
        &self.groups
    }
}

impl Display for GetMeResponseView {
    #[allow(deprecated)]
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
    #[allow(deprecated)]
    fn from(query_result: GetUserByIdQueryResultView) -> Self {
        Self {
            first_name: query_result.first_name().to_string(),
            last_name: query_result.last_name().to_string(),
            email: query_result.email().to_string(),
            phone: query_result.phone_e164(),
            phone_country: query_result
                .phone_country()
                .map(std::string::ToString::to_string),
            status: query_result.status().to_string(),
            role: String::new(),
            roles: Vec::new(),
            groups: Vec::new(),
        }
    }
}
