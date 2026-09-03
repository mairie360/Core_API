use crate::database::roles::get_roles_by_id::{GetRolesByIdQueryView, Role};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_roles_by_id_query(
    view: GetRolesByIdQueryView,
    smart_db: &SmartDatabase,
) -> Result<Vec<Role>, ApiLibError> {
    let result: Vec<Role> = smart_db.fetch_all(&view).await?;

    Ok(result)
}
