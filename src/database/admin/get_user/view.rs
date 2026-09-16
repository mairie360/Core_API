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
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(user_id as i32)],
        }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for AdminGetUserQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT first_name, last_name, email, phone_number, status, is_archived FROM users WHERE id = $1) t"
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
    pub fn new(id: i32, name: &str, description: Option<&str>) -> Self {
        Self {
            id,
            name: name.to_string(),
            description: description.map(|d| d.to_string()),
        }
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

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
    /// Numéro de téléphone, ou `null` s'il n'en a pas renseigné.
    #[schema(example = "0612345678")]
    phone_number: Option<String>,
    /// Statut du compte tel qu'il est stocké en base.
    #[schema(example = "active")]
    status: String,
    /// `true` si le compte est archivé : il ne peut plus se connecter.
    #[schema(example = false)]
    is_archived: bool,
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "User: first_name: {}, last_name: {}, email: {}, phone_number: {:?}, status: {}, is_archived: {}",
            self.first_name, self.last_name, self.email, self.phone_number, self.status, self.is_archived
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
    pub fn new(
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

    pub fn user(&self) -> &User {
        &self.user
    }

    pub fn roles(&self) -> &Vec<RoleQueryResult> {
        &self.roles
    }

    pub fn groups(&self) -> Vec<Group> {
        self.groups.clone()
    }

    pub fn sessions(&self) -> &Vec<Session> {
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
