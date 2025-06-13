use super::postgresql::postgre_interface::{create_postgre_interface, get_postgre_interface};
use crate::database::queries_result_views::QueryResult;
use crate::database::QUERY;
use std::collections::HashMap;
use std::env;
use std::future::Future;
use std::pin::Pin;
use std::sync::{LazyLock, Mutex};

static DB_INTERACE: LazyLock<Mutex<Option<DbInterface>>> =
    LazyLock::new(|| Mutex::new(Some(DbInterface::new())));

static DB_TYPES: LazyLock<HashMap<String, DatabaseType>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert(String::from("postgres"), DatabaseType::PostgreSQL);
    map
});

fn get_db_type(key: &str) -> DatabaseType {
    DB_TYPES.get(key).cloned().unwrap_or(DatabaseType::Unknown)
}

pub fn get_db_interface() -> &'static Mutex<Option<DbInterface>> {
    &DB_INTERACE
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
    fn execute_query(
        &self,
        query: Box<dyn DatabaseQueryView>,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn QueryResultView>, String>> + Send>>;
}
pub struct DbInterface {
    // db_interface: Box<dyn DatabaseInterfaceActions + Send>
}

impl DbInterface {
    pub fn new() -> Self {
        println!("Initializing database interface...");
        let db_type = env::var("DB_TYPE");
        println!("Database type: {:?}", db_type);
        match get_db_type(db_type.clone().unwrap().as_str()) {
            DatabaseType::PostgreSQL => {
                create_postgre_interface();
            }
            _ => {
                eprintln!("Unsupported database type: {}", db_type.unwrap().as_str());
                std::process::exit(1);
            }
        }
        DbInterface {}
    }

    pub async fn connect(&mut self) -> Result<String, String> {
        let mut guard = get_postgre_interface().await;
        if let Some(ref mut postgre_interface) = *guard {
            match postgre_interface.connect().await {
                Ok(message) => Ok(message),
                Err(e) => Err(e),
            }
        } else {
            Err("PostgreInterface not initialized".to_string())
        }
    }

    pub async fn disconnect(&mut self) -> Result<String, String> {
        let mut guard = get_postgre_interface().await;
        if let Some(ref mut postgre_interface) = *guard {
            match postgre_interface.disconnect().await {
                Ok(message) => Ok(message),
                Err(e) => Err(e),
            }
        } else {
            Err("PostgreInterface not initialized".to_string())
        }
    }

    pub async fn execute_query(
        &self,
        query: Box<dyn DatabaseQueryView>,
    ) -> Result<Box<dyn QueryResultView>, String> {
        println!("Executing query: {}", query.get_request());
        let guard = get_postgre_interface().await;
        if let Some(ref postgre_interface) = *guard {
            match postgre_interface.execute_query(query).await {
                Ok(result) => Ok(result),
                Err(e) => Err(e),
            }
        } else {
            Err("PostgreInterface not initialized".to_string())
        }
    }
}
