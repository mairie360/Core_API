use crate::database::db_interface::{DatabaseQueryView, QueryResultView};
use crate::database::queries_result_views::RegisterUserQueryResultView;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Client;

pub async fn register_user(
    query: Box<dyn DatabaseQueryView>,
    client: Arc<Mutex<Option<Client>>>,
) -> Result<Box<dyn QueryResultView>, String> {
    println!("Registering user with query: {}", query.get_request());
    let tmp_client = client.lock().await;
    let client = tmp_client
        .as_ref()
        .ok_or("Database client is not initialized")?;
    let result = client.execute(query.get_request().as_str(), &[]).await;
    println!("result: {:?}", result);
    match result {
        Ok(1) => {
            // You can optionally check that _row_count == 1
            Ok(Box::new(RegisterUserQueryResultView::new(Ok(()))))
        }
        Ok(_) => {
            println!("Unexpected number of rows affected, expected 1 but got more.");
            Err("Unexpected number of rows affected".to_string())
        }
        Err(e) => {
            println!("Error executing query: {}", e);
            Err(format!("Database query error: {}", e))
        }
    }
}
