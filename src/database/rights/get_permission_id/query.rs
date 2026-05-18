use crate::database::rights::get_permission_id::view::GetPermissionIdQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_permission_id_query(
    view: GetPermissionIdQueryView,
    pool: PgPool,
) -> Result<u64, DatabaseError> {
    let result: i32 = sqlx::query_scalar(&view.get_request())
        .bind(view.resource_id() as i32)
        .bind(view.action().to_string())
        .fetch_one(&pool)
        .await?;

    Ok(result as u64)
}
