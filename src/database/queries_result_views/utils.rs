use std::fmt;

/**
 * This module provides utility functions to handle query results in a database context.
 * It defines a `QueryResult` enum to encapsulate different types of results,
 */
pub enum QueryResult {
    Boolean(bool),
    Result(Result<(), String>),
}

/**
 * This function takes a `QueryResult` and returns a boolean value.
 * If the result is of type `QueryResult::Boolean`, it returns the boolean value.
 * If the result is not of type `QueryResult::Boolean`, it prints an error message and returns `false`.
 * * # Arguments
 * * `result`: A `QueryResult` enum instance.
 * # Returns
 * * A boolean value extracted from the `QueryResult`.
 */
pub fn get_boolean_from_query_result(result: QueryResult) -> bool {
    match result {
        QueryResult::Boolean(b) => b,
        _ => {
            eprintln!("Expected QueryResult::Boolean");
            false
        }
    }
}


/**
 * This function takes a `QueryResult` and returns a `Result<(), String>`.
 * If the result is of type `QueryResult::Result`, it returns the contained result.
 * If the result is not of type `QueryResult::Result`, it prints an error message and returns an error.
 * * # Arguments
 * * `result`: A `QueryResult` enum instance.
 * # Returns
 * * A `Result<(), String>` extracted from the `QueryResult`.
 */
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
