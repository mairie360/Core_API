use std::sync::{
    Mutex,
    LazyLock
};
use std::collections::HashMap;
use std::env;

static DISPLAY: LazyLock<Mutex<db_interface>> = LazyLock::new(||{
        Mutex::new(db_interface::new())
    }
);

static db_types: LazyLock<HashMap<String, DatabaseType>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert(String::from("postgres"), DatabaseType::PostgreSQL);
    map
});

pub fn get_db_interface() -> &'static Mutex<db_interface> {
    &DISPLAY
}

pub enum DatabaseType {
    PostgreSQL,
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

pub trait DatabaseInterfaceActions {
    fn connect(&self) -> Result<(), String>;
    fn disconnect(&self) -> Result<(), String>;
    fn execute_query(&self, query: &impl DatabaseQueryView) -> Result<&impl DatabaseQueryView, String>;
}

pub struct db_interface {
    pub db_name: String,
    pub db_user: String,
    pub db_password: String,
    pub db_host: String,
    pub db_port: u16,
}

impl db_interface {
    pub fn new() -> Self {
        println!("Initializing database interface...");
        let db_interface = db_interface {
            db_name: String::from(""),
            db_user: String::from(""),
            db_password: String::from(""),
            db_host: String::from(""),
            db_port: 0
        };
        match env::var("DB_TYPE") {
            Ok(db_type) => {
                println!("Detected DB_TYPE: {}", db_type);
                if let Some(db_enum) = db_types.get(&db_type) {
                } else {
                    eprintln!("Unsupported database type: {}", db_type);
                    std::process::exit(1);
                }
            },
            Err(_) => {
                eprintln!("DB_TYPE environment variable not set. Please set it to a supported database type.");
                std::process::exit(1);
            }
        }
        println!("Initializing database interface with name: {}", db_interface.db_name);
        println!("Database user: {}", db_interface.db_user);
        println!("Database host: {}", db_interface.db_host);
        println!("Database port: {}", db_interface.db_port);
        println!("Database password: {}", db_interface.db_password);
        match db_interface.connect() {
            Ok(_) => println!("Connected to database successfully."),
            Err(e) => {
                eprintln!("Error connecting to database: {}", e);
                eprintln!("Please check your database configuration.");
                std::process::exit(1);
            }
        }
        db_interface
    }

    pub fn connect(&self) -> Result<(), String> {
        // Placeholder for actual database connection logic
        Ok(())
    }

    pub fn disconnect(&self) -> Result<(), String> {
        // Placeholder for actual database disconnection logic
        Ok(())
    }
}

impl Drop for db_interface {
    fn drop(&mut self) {
        match self.disconnect() {
            Ok(_) => println!("Disconnected from database successfully."),
            Err(e) => eprintln!("Error disconnecting from database: {}", e),
        }
    }
}