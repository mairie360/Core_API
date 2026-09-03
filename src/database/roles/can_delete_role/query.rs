use crate::database::roles::can_delete_role::CanDeleteRoleQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn can_delete_role_query(
    view: CanDeleteRoleQueryView,
    smart_db: &SmartDatabase,
) -> Result<bool, ApiLibError> {
    let result: bool = smart_db.fetch_scalar(&view).await?;

    Ok(result)
}
