use crate::database::roles::get_roles_by_id::{GetRolesByIdQueryView, Role};
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_roles_by_id_query(
    view: GetRolesByIdQueryView,
    pool: PgPool,
) -> Result<Vec<Role>, DatabaseError> {
    let result: Vec<Role> = sqlx::query_as::<_, Role>(&view.get_request())
        .bind(view.id())
        .fetch_all(&pool)
        .await?;

    Ok(result)
}
