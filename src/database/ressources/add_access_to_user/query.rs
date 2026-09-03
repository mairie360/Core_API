use crate::database::ressources::add_access_to_user::view::AddAccessToUserQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn add_access_to_user_query(
    view: AddAccessToUserQueryView,
    smart_db: &SmartDatabase,
) -> Result<(), ApiLibError> {
    smart_db.execute(view).await?;

    Ok(())
}
