use crate::database::ressources::add_access_to_user::view::AddAccessToUserQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn add_access_to_user_query(
    view: AddAccessToUserQueryView,
    pool: PgPool,
) -> Result<(), DatabaseError> {
    sqlx::query(&view.get_request())
        .bind(view.user_id() as i64)
        .bind(view.ressource_type_id() as i64)
        .bind(view.ressource_instance_id() as i64)
        .bind(view.access_type_id() as i64)
        .fetch_one(&pool)
        .await?;

    Ok(())
}
