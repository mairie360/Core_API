use crate::database::postgresql::queries::AboutUserQuery;
use crate::database::queries_result_views::AboutUserQueryResultView;
use crate::database::query_views::AboutUserQueryView;
use mairie360_api_lib::database::db_interface::get_db_interface;
use mairie360_api_lib::database::errors::DatabaseError;

pub async fn about_user_query(
    view: AboutUserQueryView,
) -> Result<AboutUserQueryResultView, DatabaseError> {
    let db_guard = get_db_interface().lock().unwrap();
    let db_interface = match &*db_guard {
        Some(db) => db,
        None => {
            eprintln!("Database interface is not initialized.");
            return Err(DatabaseError::NotInitialized);
        }
    };
    let query = AboutUserQuery::new(*view.get_id());
    db_interface.execute_query(query).await
}
