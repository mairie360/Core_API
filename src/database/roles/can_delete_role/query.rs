use crate::database::roles::can_delete_role::CanDeleteRoleQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn can_delete_role_query(
    view: CanDeleteRoleQueryView,
    pool: PgPool,
) -> Result<bool, DatabaseError> {
    let result: bool = sqlx::query_scalar::<_, bool>(&view.get_request())
        .bind(view.id() as i64)
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
