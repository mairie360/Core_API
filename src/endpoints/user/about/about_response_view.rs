use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/**
 * AboutResponseView is a view model for the user about response.
 * It contains the user's first name, last name, email, phone number, and status.
 * This struct is used to serialize the response to the client.
 *
 * #properties
 * - `first_name`: The first name of the user.
 * - `last_name`: The last name of the user.
 * - `email`: The email address of the user.
 * - `phone`: The phone number of the user.
 * - `status`: The status of the user (e.g., active, inactive).
 */
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AboutResponseView {
    first_name: String,
    last_name: String,
    email: String,
    phone: String,
    status: String,
}

impl AboutResponseView {
    /**
     * Creates a new instance of `AboutResponseView`.
     *
     * # Arguments
     * - `first_name`: The first name of the user.
     * - `last_name`: The last name of the user.
     * - `email`: The email address of the user.
     * - `phone`: The phone number of the user.
     * - `status`: The status of the user.
     */
    pub fn new(
        first_name: String,
        last_name: String,
        email: String,
        phone: String,
        status: String,
    ) -> Self {
        AboutResponseView {
            first_name,
            last_name,
            email,
            phone,
            status,
        }
    }

    /**
     * Getters for the fields of `AboutResponseView`.
     * These methods allow access to the private fields of the struct.
     *
     * # Returns
     * - `&str`: A reference to the string value of the field.
     */
    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    /**
     * Getters for the fields of `AboutResponseView`.
     * These methods allow access to the private fields of the struct.
     *
     * # Returns
     * - `&str`: A reference to the string value of the field.
     */
    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    /**
     * Getters for the fields of `AboutResponseView`.
     * These methods allow access to the private fields of the struct.
     *
     * # Returns
     * - `&str`: A reference to the string value of the field.
     */
    pub fn email(&self) -> &str {
        &self.email
    }

    /**
     * Getters for the fields of `AboutResponseView`.
     * These methods allow access to the private fields of the struct.
     *
     * # Returns
     * - `&str`: A reference to the string value of the field.
     */
    pub fn phone(&self) -> &str {
        &self.phone
    }

    /**
     * Getters for the fields of `AboutResponseView`.
     * These methods allow access to the private fields of the struct.
     *
     * # Returns
     * - `&str`: A reference to the string value of the field.
     */
    pub fn status(&self) -> &str {
        &self.status
    }
}

impl Display for AboutResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AboutResponseView {{ first_name: {}, last_name: {}, email: {}, phone: {}, status: {} }}",
            self.first_name,
            self.last_name,
            self.email,
            self.phone,
            self.status
        )
    }
}
