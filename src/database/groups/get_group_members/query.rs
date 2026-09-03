use crate::database::groups::get_group_members::view::GetGroupUsersQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_group_members_query(
    view: GetGroupUsersQueryView,
    smart_db: &SmartDatabase,
) -> Result<Vec<i32>, ApiLibError> {
    let result: Vec<i32> = smart_db.fetch_all(&view).await?;

    Ok(result)
}
