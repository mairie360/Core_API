use crate::database::sessions::revoke_session_by_id::RevokeSessionByIdQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn revoke_session_by_id_query(
    view: RevokeSessionByIdQueryView,
    smart_db: &SmartDatabase,
) -> Result<(), ApiLibError> {
    smart_db.execute(view).await?;

    Ok(())
}
