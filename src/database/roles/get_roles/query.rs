use crate::database::roles::get_roles::view::{GetRolesQueryView, RoleQueryResult};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_roles_query(
    view: GetRolesQueryView,
    smart_db: &SmartDatabase,
) -> Result<Vec<RoleQueryResult>, ApiLibError> {
    let result: Vec<RoleQueryResult> = smart_db.fetch_all(&view).await?;

    Ok(result)
}
