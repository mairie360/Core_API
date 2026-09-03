use crate::database::groups::is_user_member::view::IsUserMemberQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn is_user_member_query(
    view: IsUserMemberQueryView,
    smart_db: &SmartDatabase,
) -> Result<bool, ApiLibError> {
    let result = smart_db.fetch_scalar(&view).await?;

    Ok(result)
}
