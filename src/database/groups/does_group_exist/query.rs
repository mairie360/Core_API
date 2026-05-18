use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

use crate::database::groups::does_group_exist::DoesGroupExistQuerView;

pub async fn does_group_exist_query(
    view: DoesGroupExistQuerView,
    pool: PgPool,
) -> Result<bool, DatabaseError> {
    let result: bool = sqlx::query_scalar::<_, bool>(&view.get_request())
        .bind(view.group_id() as i32)
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
