use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/**
 * This module defines the request views for the "about" endpoint of a user.
 * It includes structures for both the request body and path parameters.
 * The `AboutRequestView` is used for the request body, while `AboutPathParamRequestView`
 * is used for path parameters.
 * Both structures implement the `Display` trait for easy string representation.
 */
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AboutRequestView {
    user_id: u64,
}

impl AboutRequestView {
    /**
     * Creates a new instance of `AboutRequestView` with the specified user ID.
     *
     * # Arguments
     * * `user_id` - The ID of the user for whom the about information is being requested.
     */
    pub fn new(user_id: u64) -> Self {
        AboutRequestView { user_id }
    }

    /**
     * Returns the user ID associated with this request view.
     *
     * # Returns
     * The user ID as a `u64`.
     */
    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl Display for AboutRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AboutRequestView {{ user_id: {}}}", self.user_id)
    }
}

/**
 * This structure represents the path parameters for the "about" endpoint.
 * It includes the user ID as a path parameter.
 */
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AboutPathParamRequestView {
    pub user_id: u64,
}

impl AboutPathParamRequestView {
    /**
     * Return the user ID associated with this path parameter request view.
     *
     * # Returns
     * The user ID as a `u64`.
     */
    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl Display for AboutPathParamRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AboutRequestView {{ user_id: {} }}", self.user_id)
    }
}
