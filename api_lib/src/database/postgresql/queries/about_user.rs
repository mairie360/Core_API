use crate::database::db_interface::{DatabaseQueryView, QueryResultView};
use crate::database::queries_result_views::AboutUserQueryResultView;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Client;

pub async fn about_user(
    query: Box<dyn DatabaseQueryView>,
    client: Arc<Mutex<Option<Client>>>,
) -> Result<Box<dyn QueryResultView>, String> {
    println!("Executing about_user query: {}", query.get_request());
    let tmp_client = client.lock().await;
    let client = tmp_client
        .as_ref()
        .ok_or("Database client is not initialized")?;
    let result = client.query_one(query.get_request().as_str(), &[]).await;
    println!("Query result: {:?}", result);
    match result {
        Ok(row) => {
            let first_name: &str = row.get("first_name");
            let last_name: &str = row.get("last_name");
            let email: &str = row.get("email");
            let phone: &str = row.get("phone_number");
            let status: &str = row.get("status");
            println!(
                "User details - First Name: {}, Last Name: {}, Email: {}, Phone: {}, Status: {}",
                first_name, last_name, email, phone, status
            );

            Ok(Box::new(AboutUserQueryResultView::new(
                first_name,
                last_name,
                email,
                phone,
                status,
            )))
        }
        Err(e) => {
            eprintln!("Error executing query: {}", e);
            Err(format!("Database query error: {}", e))
        }
    }
}
