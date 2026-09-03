use crate::database::users::get_user_by_id::GetUserByIdQueryResultView;
use crate::database::users::get_user_by_id::GetUserByIdQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_user_by_id_query(
    view: GetUserByIdQueryView,
    smart_db: &SmartDatabase,
) -> Result<GetUserByIdQueryResultView, ApiLibError> {
    let result = smart_db.fetch_one(&view).await?;

    Ok(result)
}
