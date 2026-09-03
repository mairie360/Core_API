use crate::database::auth::login::LoginUserQueryResultView;
use crate::database::auth::login::LoginUserQueryView;
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn login_query(
    view: LoginUserQueryView,
    smart_db: &SmartDatabase,
) -> Result<Option<LoginUserQueryResultView>, ApiLibError> {
    match smart_db.fetch_one(&view).await {
        Ok(result) => Ok(Some(result)),
        Err(ApiLibError::Database(DbError::NotFound)) => Ok(None),
        Err(err) => Err(err),
    }
}
