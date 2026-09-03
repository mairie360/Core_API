use crate::database::sessions::revoke_previous_session::RevokePreviousSessionQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn revoke_previous_session_query(
    view: RevokePreviousSessionQueryView,
    smart_db: &SmartDatabase,
) -> Result<(), ApiLibError> {
    smart_db.execute(view).await?;

    Ok(())
}
