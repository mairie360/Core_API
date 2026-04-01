#[derive(Debug, sqlx::FromRow, PartialEq, Eq)]
pub struct LoginUserQueryResultView {
    #[sqlx(rename = "id")]
    user_id: i32,
    #[sqlx(rename = "password")]
    password: String,
}

impl LoginUserQueryResultView {
    pub fn new(user_id: i32, password: String) -> Self {
        Self { user_id, password }
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn user_id(&self) -> i32 {
        self.user_id
    }
}
