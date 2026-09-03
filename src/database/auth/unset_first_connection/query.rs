use crate::database::auth::unset_first_connection::UnsetFirstConnectionQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn unset_first_connection_query(
    view: UnsetFirstConnectionQueryView,
    smart_db: &SmartDatabase,
) -> Result<(), ApiLibError> {
    smart_db.execute(view).await?;

    Ok(())
}
