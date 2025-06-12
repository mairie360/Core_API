use crate::database::db_interface::DatabaseInterfaceActions;
use tokio_postgres::{config, Client, Error, NoTls};
use std::sync::{
    Arc, LazyLock,
};
use tokio::sync::Mutex;
use std::future::Future;
use std::pin::Pin;

use super::super::super::get_critical_env_var;

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
        Box::pin(async move { Ok(String::from("PostgreSql Disconnected")) })
        // if let Some(conn) = self.connection.take() {
        //     conn.close().await.map_err(|e| format!("Failed to disconnect: {}", e))?;
        // }
        // self.client = None;
        // Ok(())
    }

    // fn execute_query(&self, query: &impl DatabaseQueryView) -> Result<&impl DatabaseQueryView, String> {
    //     // // Implementation for executing a query
    //     // unimplemented!()
    // }
}