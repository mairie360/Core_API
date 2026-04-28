use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

use crate::database::roles::patch_role::PatchRoleQueryView;

pub async fn patch_role_query(view: PatchRoleQueryView, pool: PgPool) -> Result<(), DatabaseError> {
    eprintln!("patch_role_query: view={:?}", view);
    sqlx::query(&view.get_request())
        .bind(view.name())
        .bind(view.description())
        .bind(view.can_be_deleted())
        .bind(view.id() as i64)
        .execute(&pool)
        .await?;

    Ok(())
}
