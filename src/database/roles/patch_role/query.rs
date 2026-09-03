use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

use crate::database::roles::patch_role::PatchRoleQueryView;

pub async fn patch_role_query(
    view: PatchRoleQueryView,
    smart_db: &SmartDatabase,
) -> Result<(), ApiLibError> {
    if view.is_noop() {
        return Ok(());
    }

    smart_db.execute(view).await?;

    Ok(())
}
