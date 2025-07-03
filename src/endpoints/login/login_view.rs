use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/**
 * LoginView struct
 * This struct is used to represent the data sent by the client when logging in.
 * It contains the email and password fields.
 */
#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginView {
    email: String,
    password: String,
}

impl LoginView {
    /**
     * Returns the email of the user.
     * This method is used to retrieve the email from the LoginView struct.
     *
     * # Returns
     * A string containing the email of the user.
     */
    pub fn email(&self) -> String {
        self.email.clone()
    }

    /**
     * Returns the password of the user.
     * This method is used to retrieve the password from the LoginView struct.
     *
     * # Returns
     * A string containing the password of the user.
     */
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
