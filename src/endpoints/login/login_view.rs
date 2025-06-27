use serde::Deserialize;
use std::fmt::Display;

#[derive(Deserialize)]
pub struct LoginView {
    email: String,
    password: String,
}

impl LoginView {
    pub fn email(&self) -> String {
        self.email.clone()
    }

    pub fn password(&self) -> String {
        self.password.clone()
    }
}

impl Display for LoginView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LoginView {{ email: {}, password: {} }}",
            self.email, self.password
        )
    }
}
