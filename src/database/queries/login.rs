use crate::database::postgresql::queries::LoginUserQuery;
use crate::database::queries_result_views::LoginUserQueryResultView;
use crate::database::query_views::LoginUserQueryView;
use mairie360_api_lib::database::db_interface::get_db_interface;
use mairie360_api_lib::database::errors::DatabaseError;

pub async fn login_query(
    view: LoginUserQueryView,
) -> Result<LoginUserQueryResultView, DatabaseError> {
    let db_guard = get_db_interface().lock().unwrap();
    let db_interface = match &*db_guard {
        Some(db) => db,
        None => {
            eprintln!("Database interface is not initialized.");
            return Err(DatabaseError::NotInitialized);
        }
    };
    let query = LoginUserQuery::new(view.get_email(), view.get_password());
    db_interface.execute_query(query).await
}
