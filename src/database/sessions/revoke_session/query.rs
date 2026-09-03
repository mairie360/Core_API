use crate::database::sessions::revoke_session::RevokeSessionQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn revoke_session_query(
    view: RevokeSessionQueryView,
    smart_db: &SmartDatabase,
) -> Result<(), ApiLibError> {
    smart_db.execute(view).await?;

    Ok(())
}
