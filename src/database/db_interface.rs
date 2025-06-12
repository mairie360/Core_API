use std::sync::{
    Mutex,
    LazyLock
};
use std::future::Future;
use std::pin::Pin;
use std::collections::HashMap;
use std::env;
use super::postgresql::postgre_interface::PostgreInterface;
use crate::database::QUERY;
use crate::database::queries_result_views::QueryResult;

static DISPLAY: LazyLock<Mutex<DbInterface>> = LazyLock::new(||{
        Mutex::new(DbInterface::new())
    }
);

static DB_TYPES: LazyLock<HashMap<String, DatabaseType>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert(String::from("postgres"), DatabaseType::PostgreSQL);
    map
});

fn get_db_type(key: &str) -> DatabaseType {
    DB_TYPES
        .get(key)
        .cloned()
        .unwrap_or(DatabaseType::Unknown)
}

pub fn get_db_interface() -> &'static Mutex<DbInterface> {
    &DISPLAY
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatabaseType {
    PostgreSQL,
    Unknown,
}

pub enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
}

pub trait QueryResultView {
    fn get_result(&self) -> QueryResult;
}

pub trait DatabaseQueryView: Send {
    fn get_request(&self) -> String;
    fn get_query_type(&self) -> QUERY;
}

pub trait DatabaseInterfaceActions: Send {
    fn connect(&mut self) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>;
    fn disconnect(&mut self) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>;
    fn execute_query(&self, query: Box<dyn DatabaseQueryView>) -> Pin<Box<dyn Future<Output = Result<Box<dyn QueryResultView>, String>> + Send>>;
}
pub struct DbInterface {
    db_interface: Box<dyn DatabaseInterfaceActions + Send>
}

impl DbInterface {
    pub fn new() -> Self {
        println!("Initializing database interface...");
        let db_type = env::var("DB_TYPE");
        println!("Database type: {:?}", db_type);
        let db_interface = DbInterface {
            db_interface: match get_db_type(db_type.clone().unwrap().as_str()) {
                DatabaseType::PostgreSQL => {
                    Box::new(PostgreInterface::new())
                }
                _ => {
                    eprintln!("Unsupported database type: {}", db_type.unwrap().as_str());
                    std::process::exit(1);
                }
            },
        };
        db_interface
    }

    pub async fn connect(&mut self) -> Result<String, String> {
        match self.db_interface.connect().await {
            Ok(message) => Ok(message),
            Err(e) => Err(e),
        }
    }

    pub async fn disconnect(&mut self) -> Result<String, String> {
        // Placeholder for actual database disconnection logic
        match self.db_interface.disconnect().await {
            Ok(message) => Ok(message),
            Err(e) => Err(e),
        }
    }

    pub async fn execute_query(&self, query: Box<dyn DatabaseQueryView>) -> Result<Box<dyn QueryResultView>, String> {
        println!("Executing query: {}", query.get_request());
        // Err("Not implemented yet".to_string())
        match self.db_interface.execute_query(query).await {
            Ok(result) => Ok(result),
            Err(e) => Err(e),
        }
    }
}