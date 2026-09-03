use crate::database::groups::delete_user_from_group::DeleteUserFromGroupQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn delete_user_from_group_query(
    view: DeleteUserFromGroupQueryView,
    smart_db: &SmartDatabase,
) -> Result<(), ApiLibError> {
    smart_db.execute(view).await?;

    Ok(())
}
