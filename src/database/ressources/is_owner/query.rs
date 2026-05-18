use crate::database::ressources::is_owner::IsOwnerQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn is_owner_query(view: IsOwnerQueryView, pool: PgPool) -> Result<bool, DatabaseError> {
    let result: bool = sqlx::query_scalar::<_, bool>(&view.get_request())
        .bind(view.ressource_id() as i64)
        .bind(view.owner_id() as i64)
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
