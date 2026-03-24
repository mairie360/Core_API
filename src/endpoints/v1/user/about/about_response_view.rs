use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

use crate::database::queries_result_views::AboutUserQueryResultView;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AboutResponseView {
    first_name: String,
    last_name: String,
    email: String,
    phone: String,
    status: String,
}

impl AboutResponseView {
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

    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn phone(&self) -> &str {
        &self.phone
    }

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

impl From<AboutUserQueryResultView> for AboutResponseView {
    fn from(query_result: AboutUserQueryResultView) -> Self {
        AboutResponseView {
            first_name: query_result.first_name().to_string(),
            last_name: query_result.last_name().to_string(),
            email: query_result.email().to_string(),
            phone: query_result.phone_number().to_string(),
            status: query_result.status().to_string(),
        }
    }
}
