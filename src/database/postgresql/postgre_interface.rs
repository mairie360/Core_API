use crate::database::db_interface::DatabaseInterfaceActions;
use tokio_postgres::{NoTls, Error};
use std::sync::{
    Arc, LazyLock, Mutex
};

pub struct PostgreInterface {
    client: Option<tokio_postgres::Client>,
    connection: Option<tokio_postgres::Connection<tokio::net::TcpStream, NoTls>>,
}

impl PostgreInterface {
    pub fn new() -> Self {
        PostgreInterface {
            client: None,
            connection: None,
        }
    }
}

impl DatabaseInterfaceActions for PostgreInterface {
    fn connect(&self) -> Result<String, String> {
        Ok(String::from("PostgreSql Connected"))
        // let connection_string = "host=localhost user=postgres password=your_password dbname=your_db";
        // match tokio_postgres::connect(connection_string, NoTls).await {
        //     Ok((client, connection)) => {
        //         self.client = Some(client);
        //         self.connection = Some(connection);
        //         Ok(())
        //     }
        //     Err(e) => Err(format!("Failed to connect to database: {}", e)),
        // }
    }

    fn disconnect(&self) -> Result<String, String> {
        Ok(String::from("PostgreSql Disconnected"))
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