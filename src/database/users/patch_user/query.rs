use crate::database::users::patch_user::PatchUserQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn patch_user_query(
    view: PatchUserQueryView,
    smart_db: &SmartDatabase,
) -> Result<(), ApiLibError> {
    if view.is_noop() {
        return Ok(());
    }

    smart_db.execute(view).await?;

    Ok(())
}
