use std::fmt;

pub enum QueryResult {
    Boolean(bool),
}

impl fmt::Display for QueryResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            QueryResult::Boolean(b) => write!(f, "{}", b),
        }
    }
}