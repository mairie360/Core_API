use crate::database::groups::create_group::view::CreateGroupQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn create_group_query(
    view: CreateGroupQueryView,
    pool: PgPool,
) -> Result<i32, DatabaseError> {
    let result: i32 = sqlx::query_scalar(&view.get_request())
        .bind(view.owner_id() as i32)
        .bind(view.name())
        .bind(view.description())
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
