use serde::Deserialize;
use std::fmt::Display;
use utoipa::ToSchema;

/**
 * RegisterView is a struct that represents the data required for user registration.
 * It includes fields for first name, last name, email, password, and an optional phone
 */
#[derive(Deserialize, ToSchema)]
pub struct RegisterView {
    first_name: String,
    last_name: String,
    email: String,
    password: String,
    phone_number: Option<String>,
}

impl RegisterView {
    /**
     * # Returns:
     * - `String` containing the first name of the user.
     */
    pub fn first_name(&self) -> String {
        self.first_name.clone()
    }

    /**
     * # Returns:
     * - `String` containing the last name of the user.
     */
    pub fn last_name(&self) -> String {
        self.last_name.clone()
    }

    /**
     * # Returns:
     * - `String` containing the email of the user.
     */
    pub fn email(&self) -> String {
        self.email.clone()
    }

    /**
     * # Returns:
     * - `String` containing the password of the user.
     */
    pub fn password(&self) -> String {
        self.password.clone()
    }

    /**
     * # Returns:
     * - `Option<String>` containing the phone number of the user, if provided.
     */
    pub fn phone_number(&self) -> Option<String> {
        self.phone_number.clone()
    }
}

impl Display for RegisterView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RegisterView {{ first_name: {}, last_name: {}, email: {}, password: {}, phone_number: {:?} }}",
            self.first_name, self.last_name, self.email, self.password, self.phone_number
        )
    }
}
