use crate::database::ressources::is_owner::IsOwnerQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn is_owner_query(
    view: IsOwnerQueryView,
    smart_db: &SmartDatabase,
) -> Result<bool, ApiLibError> {
    let result: bool = smart_db.fetch_scalar(&view).await?;

    Ok(result)
}
