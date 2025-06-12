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
    pub fn first_name(&self) -> String {
        self.first_name.clone()
    }

    pub fn last_name(&self) -> String {
        self.last_name.clone()
    }

    pub fn email(&self) -> String {
        self.email.clone()
    }

    pub fn password(&self) -> String {
        self.password.clone()
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