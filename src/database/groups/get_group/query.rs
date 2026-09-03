use crate::database::groups::get_group::view::{GetGroupQuerView, Group};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_group_query(
    view: GetGroupQuerView,
    smart_db: &SmartDatabase,
) -> Result<Group, ApiLibError> {
    let result: Group = smart_db.fetch_one(&view).await?;

    Ok(result)
}
