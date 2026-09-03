use crate::database::roles::does_role_exist::view::DoesRoleExistQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn does_role_exist_query(
    view: DoesRoleExistQueryView,
    smart_db: &SmartDatabase,
) -> Result<bool, ApiLibError> {
    let result: bool = smart_db.fetch_scalar(&view).await?;

    Ok(result)
}
