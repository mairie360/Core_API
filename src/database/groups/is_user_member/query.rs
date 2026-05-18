use crate::database::groups::is_user_member::view::IsUserMemberQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn is_user_member_query(
    view: IsUserMemberQueryView,
    pool: PgPool,
) -> Result<bool, DatabaseError> {
    let result = sqlx::query_scalar::<_, bool>(&view.get_request())
        .bind(view.group_id() as i32)
        .bind(view.user_id() as i32)
        .fetch_one(&pool)
        .await?;

    Ok(result)
}
