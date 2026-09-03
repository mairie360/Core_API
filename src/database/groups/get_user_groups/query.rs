use crate::database::groups::get_group::Group;
use crate::database::groups::get_user_groups::view::GetUserGroupsQuerView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_user_groups(
    view: GetUserGroupsQuerView,
    smart_db: &SmartDatabase,
) -> Result<Vec<Group>, ApiLibError> {
    let result: Vec<Group> = smart_db.fetch_all(&view).await?;

    Ok(result)
}
