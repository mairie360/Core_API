use crate::phone::Phone;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct RegisterUserQueryView {
    first_name: String,
    last_name: String,
    email: String,
    password: String,
    /// `(phone_country, phone_number)` of the account (MAIR-480).
    phone: Option<(String, String)>,
    params: Vec<QueryParam>,
}

impl RegisterUserQueryView {
    #[must_use]
    pub fn new(
        first_name: &str,
        last_name: &str,
        email: &str,
        password: &str,
        phone: Option<&Phone>,
    ) -> Self {
        let phone = phone.map(|phone| (phone.country().to_string(), phone.national().to_string()));
        let mut params = vec![
            QueryParam::Text(first_name.to_string()),
            QueryParam::Text(last_name.to_string()),
            QueryParam::Text(email.to_string()),
            QueryParam::Text(password.to_string()),
        ];
        if let Some((country, national)) = &phone {
            params.push(QueryParam::Text(country.clone()));
            params.push(QueryParam::Text(national.clone()));
        }
        Self {
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
            password: password.to_string(),
            phone,
            params,
        }
    }

    #[must_use]
    pub fn get_first_name(&self) -> &str {
        &self.first_name
    }
    #[must_use]
    pub fn get_last_name(&self) -> &str {
        &self.last_name
    }
    #[must_use]
    pub fn get_email(&self) -> &str {
        &self.email
    }
    #[must_use]
    pub fn get_password(&self) -> &str {
        &self.password
    }
    /// National significant number of the phone, if any.
    #[must_use]
    pub fn get_phone_number(&self) -> Option<&str> {
        self.phone.as_ref().map(|(_, national)| national.as_str())
    }

    /// Country of the phone, if any.
    #[must_use]
    pub fn get_phone_country(&self) -> Option<&str> {
        self.phone.as_ref().map(|(country, _)| country.as_str())
    }
}

impl ApiRequestDto for RegisterUserQueryView {
    fn query_sql(&self) -> &'static str {
        match self.phone {
            Some(_) => {
                "INSERT INTO users (first_name, last_name, email, password, phone_country, phone_number) \
                 VALUES ($1, $2, $3, $4, $5, $6) RETURNING true"
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
    // Never print the e-mail, phone number or password: this view may end up in logs.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RegisterUserQueryView: first_name = {}, last_name = {}, email = [PROTECTED], password = [PROTECTED], phone_number = {}",
            self.first_name,
            self.last_name,
            if self.phone.is_some() { "[PROTECTED]" } else { "none" }
        )
    }
}
