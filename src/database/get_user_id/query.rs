use crate::database::get_user_id::view::GetUserIdQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_user_id_query(
    view: GetUserIdQueryView,
    smart_db: &SmartDatabase,
) -> Result<i32, ApiLibError> {
    let result = smart_db.fetch_scalar(&view).await?;

    Ok(result)
}
