use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct RegisterUserQueryView {
    first_name: String,
    last_name: String,
    email: String,
    password: String,
    phone_number: Option<String>,
    params: Vec<QueryParam>,
}

impl RegisterUserQueryView {
    pub fn new(
        first_name: &str,
        last_name: &str,
        email: &str,
        password: &str,
        phone_number: Option<&str>,
    ) -> Self {
        Self {
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
            password: password.to_string(),
            phone_number: phone_number.map(|s| s.to_string()),
            params: match phone_number {
                Some(phone_number) => vec![
                    QueryParam::Text(first_name.to_string()),
                    QueryParam::Text(last_name.to_string()),
                    QueryParam::Text(email.to_string()),
                    QueryParam::Text(password.to_string()),
                    QueryParam::Text(phone_number.to_string()),
                ],
                None => vec![
                    QueryParam::Text(first_name.to_string()),
                    QueryParam::Text(last_name.to_string()),
                    QueryParam::Text(email.to_string()),
                    QueryParam::Text(password.to_string()),
                ],
            },
        }
    }

    pub fn get_first_name(&self) -> &str {
        &self.first_name
    }
    pub fn get_last_name(&self) -> &str {
        &self.last_name
    }
    pub fn get_email(&self) -> &str {
        &self.email
    }
    pub fn get_password(&self) -> &str {
        &self.password
    }
    pub fn get_phone_number(&self) -> Option<&str> {
        self.phone_number.as_deref()
    }
}

impl ApiRequestDto for RegisterUserQueryView {
    fn query_sql(&self) -> &'static str {
        match self.phone_number {
            Some(_) => {
                "INSERT INTO users (first_name, last_name, email, password, phone_number) \
                 VALUES ($1, $2, $3, $4, $5) RETURNING true"
            }
            None => {
                "INSERT INTO users (first_name, last_name, email, password) \
                 VALUES ($1, $2, $3, $4) RETURNING true"
            }
        }
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for RegisterUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.phone_number {
            Some(_) => write!(f, "RegisterUserQueryView: first_name = {}, last_name = {}, email = {}, password = {}, phone_number = {}", self.first_name, self.last_name, self.email, self.password, self.phone_number.as_deref().unwrap()),
            None => write!(f, "RegisterUserQueryView: first_name = {}, last_name = {}, email = {}, password = {}", self.first_name, self.last_name, self.email, self.password),
        }
    }
}
