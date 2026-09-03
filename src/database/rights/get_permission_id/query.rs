use crate::database::rights::get_permission_id::view::GetPermissionIdQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_permission_id_query(
    view: GetPermissionIdQueryView,
    smart_db: &SmartDatabase,
) -> Result<u64, ApiLibError> {
    let result: i32 = smart_db.fetch_scalar(&view).await?;

    Ok(result as u64)
}
