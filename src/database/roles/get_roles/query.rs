use crate::database::roles::get_roles::view::{GetRolesQueryView, RoleQueryResult};
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_roles_query(
    view: GetRolesQueryView,
    pool: PgPool,
) -> Result<Vec<RoleQueryResult>, DatabaseError> {
    let result: Vec<RoleQueryResult> = sqlx::query_as::<_, RoleQueryResult>(&view.get_request())
        .fetch_all(&pool)
        .await?;

    Ok(result)
}
