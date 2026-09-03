use crate::database::groups::create_group::view::CreateGroupQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn create_group_query(
    view: CreateGroupQueryView,
    smart_db: &SmartDatabase,
) -> Result<i32, ApiLibError> {
    let result: i32 = smart_db.fetch_scalar(&view).await?;

    Ok(result)
}
