use crate::database::db_interface::{DatabaseInterfaceActions, DatabaseQueryView, QueryResultView};
use crate::database::queries::QUERY::*;
use super::super::super::get_critical_env_var;
use super::queries::*;

use tokio_postgres::{Client, NoTls};
use std::sync::{
    Arc,
    LazyLock,
};
use tokio::sync::Mutex;
use std::future::Future;
use std::pin::Pin;
static POSTGRESQL_INTERFACE: LazyLock<Mutex<Option<PostgreInterface>>> = LazyLock::new(|| Mutex::new(Some(PostgreInterface::new())));

pub async fn create_postgre_interface() {
    let mut guard = POSTGRESQL_INTERFACE.lock().await;
    if guard.is_none() {
        *guard = Some(PostgreInterface::new());
    }
}

pub async fn get_postgre_interface() -> tokio::sync::MutexGuard<'static, Option<PostgreInterface>> {
    POSTGRESQL_INTERFACE.lock().await
}

pub struct PostgreInterface {
    db_name: String,
    db_user: String,
    db_password: String,
    db_host: String,
    client: Arc<Mutex<Option<Client>>>
}

impl PostgreInterface {
    pub fn new() -> Self {
        PostgreInterface {
            db_name: get_critical_env_var("DB_NAME"),
            db_user: get_critical_env_var("DB_USER"),
            db_password: get_critical_env_var("DB_PASSWORD"),
            db_host: get_critical_env_var("DB_HOST"),
            client: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_client(&self) -> Arc<Mutex<Option<Client>>> {
        self.client.clone()
    }
}

impl DatabaseInterfaceActions for PostgreInterface {
    fn connect(&mut self) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> {
    let client_ref = self.client.clone();
    let config = format!(
            "host={} user={} password={} dbname={}",
            self.db_host, self.db_user, self.db_password, self.db_name
        );

        println!("Connecting to PostgreSQL with config: {}", config);
        Box::pin(async move {
            let (client, connection) = tokio_postgres::connect(
                config.as_str(),
                NoTls
            )
            .await
            .map_err(|e| format!("Failed to connect: {}", e))?;

            // Spawn the connection future
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("Connection error: {}", e);
                }
            });

            *client_ref.lock().await = Some(client);

            Ok("PostgreSQL Connected".to_string())
        })
    }

    fn disconnect(&mut self) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> {
        Box::pin(async move { Ok(String::from("PostgreSql Disconnected"))})
    }

    fn execute_query(&self, query: Box<dyn DatabaseQueryView> ) -> Pin<Box<dyn Future<Output = Result<Box<dyn QueryResultView>, String>> + Send>> {
        let client = self.get_client();
        Box::pin(async move {
            println!("Executing query: {}", query.get_query_type());
            match query.get_query_type() {
                DoesUserExistByEmail => {
                    does_user_exist_by_email(query, client).await
                }
                RegisterUser => {
                    register_user(query, client).await
                }
                UnknownQuery => {
                    Err(format!("Unsupported query type: {}", query.get_query_type()))
                }
            }
        })
    }
}