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
    // let tmp_client = client.lock().await;
    // let client = tmp_client
    //     .as_ref()
    //     .ok_or("Database client is not initialized")?;
    // let result = client.query_one(query.get_request().as_str(), &[]).await;
    // match result {
    //     Ok(row) => {
    //         let exists: bool = row.get(0);
            Ok(Box::new(AboutUserQueryResultView::new()))
    //     }
    //     Err(e) => {
    //         eprintln!("Error executing query: {}", e);
    //         Err(format!("Database query error: {}", e))
    //     }
    // }
}
