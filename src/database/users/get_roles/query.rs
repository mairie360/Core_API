use crate::database::users::get_roles::GetUserRolesQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_user_roles_query(
    view: GetUserRolesQueryView,
    smart_db: &SmartDatabase,
) -> Result<Vec<i32>, ApiLibError> {
    let result = smart_db.fetch_all(&view).await?;

    Ok(result)
}
