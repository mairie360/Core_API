use crate::database::auth::is_first_time::IsFirstTimeQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn is_first_time_query(
    view: IsFirstTimeQueryView,
    pool: PgPool,
) -> Result<bool, DatabaseError> {
    let result = sqlx::query_scalar::<_, bool>(&view.get_request())
        .bind(view.user_id() as i32)
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
