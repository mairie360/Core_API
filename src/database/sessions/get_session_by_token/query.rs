use crate::database::sessions::get_session_by_token::GetSessionByTokenQueryView;
use crate::database::sessions::Session;
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_session_by_token_query(
    view: GetSessionByTokenQueryView,
    smart_db: &SmartDatabase,
) -> Result<Option<Session>, ApiLibError> {
    match smart_db.fetch_one(&view).await {
        Ok(result) => Ok(Some(result)),
        Err(ApiLibError::Database(DbError::NotFound)) => Ok(None),
        Err(err) => Err(err),
    }
}
