use crate::database::ressources::remove_access::view::RemoveAccessQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn remove_access_query(
    view: RemoveAccessQueryView,
    smart_db: &SmartDatabase,
) -> Result<(), ApiLibError> {
    smart_db.execute(view).await?;

    Ok(())
}
