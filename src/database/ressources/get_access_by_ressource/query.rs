use crate::database::ressources::get_access_by_ressource::{Access, GetAccessByRessourceQueryView};
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_access_by_ressource(
    view: GetAccessByRessourceQueryView,
    pool: PgPool,
) -> Result<Vec<Access>, DatabaseError> {
    let result: Vec<Access> = sqlx::query_as(&view.get_request())
        .bind(view.resource_id() as i32)
        .fetch_all(&pool)
        .await?;

    Ok(result)
}
