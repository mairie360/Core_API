use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct RegisterUserQueryView {
    first_name: String,
    last_name: String,
    email: String,
    password: String,
    phone_number: Option<String>,
}

impl RegisterUserQueryView {
    pub fn new(
        first_name: String,
        last_name: String,
        email: String,
        password: String,
        phone_number: Option<String>,
    ) -> Self {
        Self {
            first_name,
            last_name,
            email,
            password,
            phone_number,
        }
    }

    pub fn get_first_name(&self) -> &String {
        &self.first_name
    }
    pub fn get_last_name(&self) -> &String {
        &self.last_name
    }
    pub fn get_email(&self) -> &String {
        &self.email
    }
    pub fn get_password(&self) -> &String {
        &self.password
    }
    pub fn get_phone_number(&self) -> Option<String> {
        self.phone_number.clone()
    }
}

impl DatabaseQueryView for RegisterUserQueryView {
    fn get_request(&self) -> String {
        match self.phone_number {
            // Avec numéro de téléphone : 5 arguments
            Some(_) => "INSERT INTO users (first_name, last_name, email, password, phone_number) \
                        VALUES ($1, $2, $3, $4, $5)"
                .to_string(),

            // Sans numéro de téléphone : 4 arguments
            None => "INSERT INTO users (first_name, last_name, email, password) \
                    VALUES ($1, $2, $3, $4)"
                .to_string(),
        }
    }

    fn get_raw_request(&self) -> String {
        match self.phone_number {
            Some(ref phone) => format!(
                "INSERT INTO users (first_name, last_name, email, password, phone_number) VALUES ('{}', '{}', '{}', '{}', '{}')",
                self.first_name, self.last_name, self.email, self.password, phone
            ),
            None => format!(
                "INSERT INTO users (first_name, last_name, email, password) VALUES ('{}', '{}', '{}', '{}')",
                self.first_name, self.last_name, self.email, self.password
            ),
        }
    }
}

impl Display for RegisterUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RegisterUserQueryView: email = {}", self.email)
    }
}
