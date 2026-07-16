use crate::database::users::get_user_by_id::GetUserByIdQueryResultView;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct GetMeResponseView {
    first_name: String,
    last_name: String,
    email: String,
    phone: Option<String>,
    status: String,
}

impl GetMeResponseView {
    pub fn new(
        first_name: String,
        last_name: String,
        email: String,
        phone: Option<String>,
        status: String,
    ) -> Self {
        GetMeResponseView {
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

    pub fn phone(&self) -> Option<&str> {
        self.phone.as_deref()
    }

    pub fn status(&self) -> &str {
        &self.status
    }
}

impl Display for GetMeResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetMeResponseView {{ first_name: {}, last_name: {}, email: {}, phone: {:?}, status: {} }}",
            self.first_name,
            self.last_name,
            self.email,
            self.phone,
            self.status
        )
    }
}

impl From<GetUserByIdQueryResultView> for GetMeResponseView {
    fn from(query_result: GetUserByIdQueryResultView) -> Self {
        GetMeResponseView {
            first_name: query_result.first_name().to_string(),
            last_name: query_result.last_name().to_string(),
            email: query_result.email().to_string(),
            phone: query_result.phone_number().map(|p| p.to_string()),
            status: query_result.status().to_string(),
        }
    }
}
