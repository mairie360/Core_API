use crate::database::queries::roles::delete_role::DeleteRoleQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn delete_role_query(
    view: DeleteRoleQueryView,
    pool: PgPool,
) -> Result<(), DatabaseError> {
    sqlx::query(&view.get_request())
        .bind(view.id() as i64)
        .execute(&pool)
        .await?;

    Ok(())
}
