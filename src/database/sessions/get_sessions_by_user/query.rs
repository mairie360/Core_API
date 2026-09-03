use crate::database::sessions::get_sessions_by_user::GetSessionsByUserQueryView;
use crate::database::sessions::Session;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_sessions_by_user_query(
    view: GetSessionsByUserQueryView,
    smart_db: &SmartDatabase,
) -> Result<Vec<Session>, ApiLibError> {
    let result: Vec<Session> = smart_db.fetch_all(&view).await?;

    Ok(result)
}
