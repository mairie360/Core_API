use crate::database::sessions::get_sessions::GetSessionsQueryView;
use crate::database::sessions::Session;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_sessions_query(
    view: GetSessionsQueryView,
    smart_db: &SmartDatabase,
) -> Result<Vec<Session>, ApiLibError> {
    let result = smart_db.fetch_all(&view).await?;

    Ok(result)
}
