use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;


#[derive(Serialize, Deserialize, ToSchema)]
pub struct AboutRequestView {
    user_id: u64,
    jwt: String,
}

impl AboutRequestView {
    pub fn new(user_id: u64, jwt: String) -> Self {
        AboutRequestView { user_id, jwt }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn jwt(&self) -> &str {
        &self.jwt
    }
}

impl Display for AboutRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AboutRequestView {{ user_id: {}, jwt: {} }}",
            self.user_id, self.jwt
        )
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AboutPathParamRequestView {
    pub user_id: u64,
}

impl AboutPathParamRequestView {
    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl Display for AboutPathParamRequestView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AboutRequestView {{ user_id: {} }}",
            self.user_id
        )
    }
}
