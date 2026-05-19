use crate::database::get_user_id::view::GetUserIdQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_user_id_query(
    view: GetUserIdQueryView,
    pool: PgPool,
) -> Result<i32, DatabaseError> {
    let result = sqlx::query_scalar(&view.get_request())
        .bind(view.email())
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
