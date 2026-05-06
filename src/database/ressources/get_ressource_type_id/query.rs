use crate::database::ressources::get_ressource_type_id::GetRessourceTypeIdQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_ressource_type_id_query(
    view: GetRessourceTypeIdQueryView,
    pool: PgPool,
) -> Result<u64, DatabaseError> {
    let result: i32 = sqlx::query_scalar(&view.get_request())
        .bind(view.ressource_type())
        .fetch_one(&pool)
        .await?;

    Ok(result as u64)
}
