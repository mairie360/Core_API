use crate::database::auth::is_first_time::IsFirstTimeQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn is_first_time_query(
    view: IsFirstTimeQueryView,
    smart_db: &SmartDatabase,
) -> Result<bool, ApiLibError> {
    let result = smart_db.fetch_scalar(&view).await?;

    Ok(result)
}
