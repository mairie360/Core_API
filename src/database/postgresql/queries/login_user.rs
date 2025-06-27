use crate::database::db_interface::{DatabaseQueryView, QueryResultView};
use crate::database::queries_result_views::LoginUserQueryResultView;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Client;

pub async fn login_user(
    query: Box<dyn DatabaseQueryView>,
    client: Arc<Mutex<Option<Client>>>,
) -> Result<Box<dyn QueryResultView>, String> {
    println!("Login user with query: {}", query.get_request());
    let tmp_client = client.lock().await;
    let client = tmp_client
        .as_ref()
        .ok_or("Database client is not initialized")?;
    let result = client.query_one(query.get_request().as_str(), &[]).await;
    println!("Login user result: {:?}", result);
    match result {
        Ok(row) => {
            match row.columns().first() {
                Some(column) => {
                    match column.column_id() {
                        Some(column_id) => {
                            let user_id: u64 = column_id as u64;
                            println!("User ID: {}", user_id);
                            Ok(Box::new(LoginUserQueryResultView::new(Ok(user_id))))
                        }
                        None => { eprintln!("Column ID not available");
                            Err("Column ID not available".to_string())
                        }
                    }
                }
                None => {
                    eprintln!("No columns found");
                    Err("No columns found in the result".to_string())
                }
            }
        }
        Err(e) => {
            println!("Error executing query: {}", e);
            Err(format!("Database query error: {}", e))
        }
    }
}
