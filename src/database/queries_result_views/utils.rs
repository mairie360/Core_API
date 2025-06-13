use std::fmt;

pub enum QueryResult {
    Boolean(bool),
    Result(Result<(), String>),
}

pub fn get_boolean_from_query_result(result: QueryResult) -> bool {
    match result {
        QueryResult::Boolean(b) => b,
        _ => {
            eprintln!("Expected QueryResult::Boolean");
            false
        }
    }
}

pub fn get_result_from_query_result(result: QueryResult) -> Result<(), String> {
    match result {
        QueryResult::Result(res) => res,
        _ => {
            eprintln!("Expected QueryResult::Result");
            Err("Expected QueryResult::Result".to_string())
        }
    }
}

impl fmt::Display for QueryResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            QueryResult::Boolean(b) => write!(f, "{}", b),
            QueryResult::Result(res) => match res {
                Ok(_) => write!(f, "Success"),
                Err(e) => write!(f, "Error: {}", e),
            },
        }
    }
}
