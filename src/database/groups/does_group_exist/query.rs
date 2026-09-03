use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

use crate::database::groups::does_group_exist::DoesGroupExistQuerView;

pub async fn does_group_exist_query(
    view: DoesGroupExistQuerView,
    smart_db: &SmartDatabase,
) -> Result<bool, ApiLibError> {
    let result: bool = smart_db.fetch_scalar(&view).await?;

    Ok(result)
}
