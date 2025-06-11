use serde::Deserialize;
use std::fmt::Display;

#[derive(Deserialize)]
pub struct RegisterView {
    first_name: String,
    last_name: String,
    email: String,
    password: String,
}

impl RegisterView {
    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

impl Display for RegisterView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RegisterView {{\n\tfirst_name: \"{}\",\n\tlast_name: \"{}\",\n\temail: \"{}\",\n\tpassword: \"{}\"\n}}",
            self.first_name, self.last_name, self.email, self.password
        )
    }
}