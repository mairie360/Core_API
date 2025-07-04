use std::fmt::Display;

/**
 * This module defines the queries used in the database.
 * Each query is represented as an enum variant.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QUERY {
    DoesUserExistByEmail,
    RegisterUser,
    UnknownQuery,
}

impl Display for QUERY {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QUERY::DoesUserExistByEmail => write!(f, "DoesUserExistByEmail"),
            QUERY::RegisterUser => write!(f, "RegisterUser"),
            QUERY::UnknownQuery => write!(f, "UnknownQuery"),
        }
    }
}
