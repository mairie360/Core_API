use crate::database::roles::create_role::view::CreateRoleQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn create_role_query(
    view: CreateRoleQueryView,
    smart_db: &SmartDatabase,
) -> Result<(), ApiLibError> {
    smart_db.execute(view).await?;
    Ok(())
}
