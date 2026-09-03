use crate::database::sessions::get_active_sessions::GetActiveSessionsQueryView;
use crate::database::sessions::Session;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_active_sessions_query(
    view: GetActiveSessionsQueryView,
    smart_db: &SmartDatabase,
) -> Result<Vec<Session>, ApiLibError> {
    let result: Vec<Session> = smart_db.fetch_all(&view).await?;

    Ok(result)
}
