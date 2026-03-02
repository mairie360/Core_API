use crate::database::postgresql::queries::RegisterUserQuery;
use crate::database::queries_result_views::RegisterUserQueryResultView;
use crate::database::query_views::RegisterUserQueryView;
use mairie360_api_lib::database::db_interface::get_db_interface;
use mairie360_api_lib::database::errors::DatabaseError;

pub async fn register_query(
    view: RegisterUserQueryView,
) -> Result<RegisterUserQueryResultView, DatabaseError> {
    let db_guard = get_db_interface().lock().unwrap();
    let db_interface = match &*db_guard {
        Some(db) => db,
        None => {
            eprintln!("Database interface is not initialized.");
            return Err(DatabaseError::NotInitialized);
        }
    };
    let query = RegisterUserQuery::new(
        view.get_first_name(),
        view.get_last_name(),
        view.get_email(),
        view.get_password(),
        view.get_phone_number(),
    );
    db_interface.execute_query(query).await
}
