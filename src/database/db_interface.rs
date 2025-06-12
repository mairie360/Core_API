use std::sync::{
    Mutex,
    LazyLock
};
use std::collections::HashMap;
use std::env;
use super::postgresql::postgre_interface::PostgreInterface;

use super::super::get_critical_env_var;

static DISPLAY: LazyLock<Mutex<db_interface>> = LazyLock::new(||{
        Mutex::new(db_interface::new())
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

pub fn get_db_interface() -> &'static Mutex<db_interface> {
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
    fn set_result(&mut self, result: String) -> Result<(), String>;
}

pub trait DatabaseQueryView {
    fn get_request(&self) -> Result<String, String>;
}

pub trait DatabaseInterfaceActions: Send {
    fn connect(&self) -> Result<String, String>;
    fn disconnect(&self) -> Result<String, String>;
    // fn execute_query(&self, query: &impl DatabaseQueryView) -> Result<&impl DatabaseQueryView, String>;
}

pub struct db_interface {
    db_name: String,
    db_user: String,
    db_password: String,
    db_host: String,
    db_interface: Box<dyn DatabaseInterfaceActions + Send>,
}

impl db_interface {
    pub fn new() -> Self {
        println!("Initializing database interface...");
        let db_type = env::var("DB_TYPE");
        println!("Database type: {:?}", db_type);
        let db_interface = db_interface {
            db_name: get_critical_env_var("DB_NAME"),
            db_user: get_critical_env_var("DB_USER"),
            db_password: get_critical_env_var("DB_PASSWORD"),
            db_host: get_critical_env_var("DB_HOST"),
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
        println!("Initializing database interface with name: {}", db_interface.db_name);
        println!("Database user: {}", db_interface.db_user);
        println!("Database host: {}", db_interface.db_host);
        println!("Database password: {}", db_interface.db_password);
        match db_interface.connect() {
            Ok(val) => {
                println!("{}", val);
            }
            Err(e) => {
                eprintln!("Error connecting to database: {}", e);
                eprintln!("Please check your database configuration.");
                std::process::exit(1);
            }
        }
        db_interface
    }

    pub fn connect(&self) -> Result<String, String> {
        // Placeholder for actual database connection logic
        self.db_interface.connect()
    }

    pub fn disconnect(&self) -> Result<String, String> {
        // Placeholder for actual database disconnection logic
        self.db_interface.disconnect()
    }
}

impl Drop for db_interface {
    fn drop(&mut self) {
        match self.disconnect() {
            Ok(val) => {
                println!("{}", val);
            }
            Err(e) => {
                eprintln!("Error disconnecting from database: {}", e);
                std::process::exit(1);
            }
        }
    }
}