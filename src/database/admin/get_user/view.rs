use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::database::{roles::get_roles_by_id::Role, sessions::Session};

pub struct AdminGetUserQueryView {
    user_id: u64,
}

impl AdminGetUserQueryView {
    pub fn new(user_id: u64) -> Self {
        Self { user_id }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl DatabaseQueryView for AdminGetUserQueryView {
    fn get_request(&self) -> String {
        "SELECT first_name, last_name, email, phone_number, status, is_archived FROM users WHERE id = $1".to_string()
    }
}

impl Display for AdminGetUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AdminGetUserQueryView: user_id: {}", self.user_id)
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize, sqlx::FromRow)]
pub struct RoleQueryResult {
    id: i32,
    name: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize, sqlx::FromRow)]
pub struct User {
    id: i32,
    first_name: String,
    last_name: String,
    email: String,
    phone_number: String,
    status: String,
    is_archived: bool,
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "User: id: {}, first_name: {}, last_name: {}, email: {}, phone_number: {}, status: {}, is_archived: {}",
            self.id, self.first_name, self.last_name, self.email, self.phone_number, self.status, self.is_archived
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AdminGetUserQueryResultView {
    user: User,
    roles: Vec<Role>,
    sessions: Vec<Session>,
}

impl AdminGetUserQueryResultView {
    pub fn new(user: User, roles: Vec<Role>, sessions: Vec<Session>) -> Self {
        Self {
            user,
            roles,
            sessions,
        }
    }

    pub fn user(&self) -> &User {
        &self.user
    }

    pub fn roles(&self) -> &Vec<Role> {
        &self.roles
    }

    pub fn sessions(&self) -> &Vec<Session> {
        &self.sessions
    }
}

impl Display for AdminGetUserQueryResultView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AdminGetUserQueryResultView: user: {:?}, roles: {:?}, sessions: {:?}",
            self.user, self.roles, self.sessions
        )
    }
}
