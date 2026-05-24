use crate::database::users::get_roles::GetUserRolesdQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_user_roles_query(
    view: GetUserRolesdQueryView,
    pool: PgPool,
) -> Result<Vec<i32>, DatabaseError> {
    let result = sqlx::query_scalar::<_, i32>(&view.get_request())
        .bind(view.get_id() as i32)
        .fetch_all(&pool)
        .await?;

    Ok(result)
}
