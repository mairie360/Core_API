use crate::database::groups::get_group::view::{GetGroupsQuerView, Group};
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_group(view: GetGroupsQuerView, pool: PgPool) -> Result<Group, DatabaseError> {
    let result: Group = sqlx::query_as(&view.get_request())
        .bind(view.group_id() as i32)
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
