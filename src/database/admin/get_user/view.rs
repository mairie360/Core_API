use crate::database::ids::id_to_sql;
use crate::database::{groups::get_group::Group, sessions::Session};
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(serde::Deserialize)]
pub struct AdminGetUserQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl AdminGetUserQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(id_to_sql(user_id))],
        }
    }

    #[must_use]
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for AdminGetUserQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT first_name, last_name, email, phone_number, phone_country, status, is_archived FROM users WHERE id = $1) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for AdminGetUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AdminGetUserQueryView: user_id: {}", self.user_id)
    }
}

#[derive(ToSchema, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleQueryResult {
    id: i32,
    name: String,
    description: Option<String>,
}

impl RoleQueryResult {
    #[must_use]
    pub fn new(id: i32, name: &str, description: Option<&str>) -> Self {
        Self {
            id,
            name: name.to_string(),
            description: description.map(std::string::ToString::to_string),
        }
    }

    #[must_use]
    pub const fn id(&self) -> i32 {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// Informations de profil d'un utilisateur, telles que vues par un administrateur.
#[derive(ToSchema, Debug, Deserialize, Eq, PartialEq, Serialize, Clone)]
pub struct User {
    /// Prénom de l'utilisateur.
    #[schema(example = "Jean")]
    first_name: String,
    /// Nom de famille de l'utilisateur.
    #[schema(example = "Dupont")]
    last_name: String,
    /// Adresse e-mail, unique sur la plateforme.
    #[schema(format = Email, example = "jean.dupont@mairie360.fr")]
    email: String,
    /// Phone number in E.164 (`+33612345678`), or `null` when the user has none. A legacy number
    /// that could not be attributed to a country (MAIR-480) is returned as stored, digits only,
    /// with `phone_country` `null`.
    #[schema(example = "+33612345678")]
    phone_number: Option<String>,
    /// ISO 3166-1 alpha-2 code of the phone number's country, or `null` when there is no phone.
    #[schema(example = "FR")]
    #[serde(default)]
    phone_country: Option<String>,
    /// Statut du compte tel qu'il est stocké en base.
    #[schema(example = "active")]
    status: String,
    /// `true` si le compte est archivé : il ne peut plus se connecter.
    #[schema(example = false)]
    is_archived: bool,
}

impl User {
    /// Replaces the stored national number by its E.164 form (MAIR-480): the rows are read with
    /// the national number of `users.phone_number`, the API answers in E.164.
    #[must_use]
    pub fn with_e164_phone(mut self) -> Self {
        self.phone_number =
            crate::phone::display(self.phone_country.as_deref(), self.phone_number.as_deref());
        self
    }
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "User: first_name: {}, last_name: {}, email: [PROTECTED], phone_number: [PROTECTED], status: {}, is_archived: {}",
            self.first_name, self.last_name, self.status, self.is_archived
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AdminGetUserQueryResultView {
    user: User,
    roles: Vec<RoleQueryResult>,
    groups: Vec<Group>,
    sessions: Vec<Session>,
}

impl AdminGetUserQueryResultView {
    #[must_use]
    pub const fn new(
        user: User,
        roles: Vec<RoleQueryResult>,
        groups: Vec<Group>,
        sessions: Vec<Session>,
    ) -> Self {
        Self {
            user,
            roles,
            groups,
            sessions,
        }
    }

    #[must_use]
    pub const fn user(&self) -> &User {
        &self.user
    }

    #[must_use]
    pub const fn roles(&self) -> &Vec<RoleQueryResult> {
        &self.roles
    }

    #[must_use]
    pub fn groups(&self) -> Vec<Group> {
        self.groups.clone()
    }

    #[must_use]
    pub const fn sessions(&self) -> &Vec<Session> {
        &self.sessions
    }
}

impl Display for AdminGetUserQueryResultView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AdminGetUserQueryResultView: user: {:?}, roles: {:?}, groups: {:?}, sessions: {:?}",
            self.user, self.roles, self.groups, self.sessions
        )
    }
}
