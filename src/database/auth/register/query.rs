use crate::database::auth::register::RegisterUserQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn register_query(
    view: RegisterUserQueryView,
    smart_db: &SmartDatabase,
) -> Result<bool, ApiLibError> {
    let result = smart_db.fetch_scalar(&view).await?;

    Ok(result)
}
