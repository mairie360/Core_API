use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

use crate::database::auth::change_password::ChangePasswordQueryView;

pub async fn change_password_query(
    view: ChangePasswordQueryView,
    smart_db: &SmartDatabase,
) -> Result<(), ApiLibError> {
    smart_db.execute(view).await?;

    Ok(())
}
