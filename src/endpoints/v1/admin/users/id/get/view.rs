use crate::database::users::get_user_by_id::GetUserByIdQueryResultView;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema, Serialize)]
pub struct GetUserResultView {
    first_name: String,
    last_name: String,
    email: String,
    phone_number: Option<String>,
}

impl Display for GetUserResultView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetUserResultView {{ first_name: {}, last_name: {}, email: {}, phone_number: {:?} }}",
            self.first_name, self.last_name, self.email, self.phone_number
        )
    }
}

impl From<GetUserByIdQueryResultView> for GetUserResultView {
    fn from(value: GetUserByIdQueryResultView) -> Self {
        Self {
            first_name: value.first_name().to_string(),
            last_name: value.last_name().to_string(),
            email: value.email().to_string(),
            phone_number: Some(value.phone_number().to_string()),
        }
    }
}
