/**
 * View for the result of a login user query.
 * This view is used to return the number of users logged in.
 * It implements the QueryResultView trait.
 * It contains a single field `user_id` which is the ID of the user that has logged in.
 */
#[derive(Debug, sqlx::FromRow, PartialEq, Eq)]
pub struct LoginUserQueryResultView {
    #[sqlx(rename = "id")]
    user_id: i32,
    #[sqlx(rename = "password")]
    password: String,
}

impl LoginUserQueryResultView {
    /**
     * Creates a new instance of `LoginUserQueryResultView`.
     *
     * # Arguments
     *
     * * `user_id` - The ID of the user that has logged in.
     * * `password` - The password of the user that has logged in.
     */
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
