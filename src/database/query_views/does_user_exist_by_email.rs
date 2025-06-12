use crate::database::db_interface::DatabaseQueryView;
use crate::database::QUERY;

pub struct DoesUserExistByEmailQueryView {
    email: String,
    query: QUERY,
}

impl DoesUserExistByEmailQueryView {
    pub fn new(email: String) -> Self {
        Self { email, query: QUERY::DoesUserExistByEmail }
    }

    pub fn get_email(&self) -> &String {
        &self.email
    }
}

impl DatabaseQueryView for DoesUserExistByEmailQueryView {
    fn get_request(&self) -> String {
        format!(
            "SELECT COUNT(*) FROM users WHERE email = '{}'",
            self.email
        )
    }

    fn get_query_type(&self) -> QUERY {
        self.query
    }
}