use crate::database::db_interface::DatabaseQueryView;
use crate::database::QUERY;
use std::fmt::Display;

pub struct DoesUserExistByIdQueryView {
    id: u64,
    query: QUERY,
}

impl DoesUserExistByIdQueryView {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            query: QUERY::DoesUserExistById,
        }
    }

    pub fn get_id(&self) -> &u64 {
        &self.id
    }
}

impl DatabaseQueryView for DoesUserExistByIdQueryView {
    fn get_request(&self) -> String {
        format!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = '{}')",
            self.id
        )
    }

    fn get_query_type(&self) -> QUERY {
        self.query
    }
}

impl Display for DoesUserExistByIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DoesUserExistByIdQueryView: id = {}", self.id)
    }
}
