use crate::database::queries_result_views::DoesUserExistByEmailQueryResultView;
use crate::database::db_interface::{DatabaseQueryView, QueryResultView};
use std::sync::{
    Arc,
};
use tokio::sync::Mutex;
use tokio_postgres::Client;

pub async fn does_user_exist_by_email(query: Box<dyn DatabaseQueryView>, client: Arc<Mutex<Option<Client>>>) -> Result<Box<dyn QueryResultView>, String> {
    let tmp_client = client.lock().await;
    let client = tmp_client.as_ref().ok_or("Database client is not initialized")?;
    let result = client.query_one(query.get_request().as_str(), &[]).await;
    match result {
        Ok(row) => {
            let exists: bool = row.get(0);
            Ok(Box::new(DoesUserExistByEmailQueryResultView::new(exists)))
        }
        Err(e) => {
            println!("Error executing query: {}", e);
            Err(format!("Database query error: {}", e))
        }
    }
    // Ok(Box::new(DoesUserExistByEmailQueryResultView::new(false)))
}