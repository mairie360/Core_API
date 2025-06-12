use std::sync::{
    Arc,
    Mutex,
    LazyLock
};
use std::future::Future;
use std::pin::Pin;
use std::collections::HashMap;
use std::env;
use super::postgresql::postgre_interface::PostgreInterface;

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
    fn connect(&mut self) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>;
    fn disconnect(&mut self) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>;
    // fn execute_query(&self, query: &impl DatabaseQueryView) -> Result<&impl DatabaseQueryView, String>;
}

pub struct db_interface {
    db_interface: Box<dyn DatabaseInterfaceActions + Send>
}

// async fn connect_interface(interface: Arc<Mutex<dyn DatabaseInterfaceActions + Send>>) -> Result<String, String> {
//     let mut locked_interface = interface.lock().map_err(|_| "Failed to lock database interface".to_string())?;
//     locked_interface.connect().await
// }

impl db_interface {
    pub fn new() -> Self {
        println!("Initializing database interface...");
        let db_type = env::var("DB_TYPE");
        println!("Database type: {:?}", db_type);
        let db_interface = db_interface {
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

    pub async fn connect(&mut self){
        match self.db_interface.connect().await {
            Ok(message) => println!("{}", message),
            Err(e) => eprintln!("{}", e),
        }
    }

    pub async fn disconnect(&mut self) {
        // Placeholder for actual database disconnection logic
        match self.db_interface.disconnect().await {
            Ok(message) => println!("{}", message),
            Err(e) => eprintln!("{}", e),
        }
    }
}