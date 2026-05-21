use crate::database::users::get_user_by_id::GetUserByIdQueryResultView;
use crate::database::users::get_user_by_id::GetUserByIdQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use mairie360_api_lib::database::queries::QueryError;
use sqlx::PgPool;

pub async fn get_user_by_id_query(
    view: GetUserByIdQueryView,
    pool: PgPool,
) -> Result<GetUserByIdQueryResultView, DatabaseError> {
    let result = sqlx::query_as::<_, GetUserByIdQueryResultView>(&view.get_request())
        .bind(view.get_id() as i32)
        .fetch_optional(&pool)
        .await?;

    result.ok_or(DatabaseError::Query(QueryError::InvalidId(
        "User ID not found".to_string(),
    )))
}
