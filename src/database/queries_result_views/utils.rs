use std::fmt;

pub enum QueryResult {
    Boolean(bool),
}

pub fn get_boolean_from_query_result(result: QueryResult) -> bool {
    match result {
        QueryResult::Boolean(b) => b,
    }
}

impl fmt::Display for QueryResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            QueryResult::Boolean(b) => write!(f, "{}", b),
        }
    }
}