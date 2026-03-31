use crate::database::queries_result_views::Session;
use crate::database::query_views::GetSessionByTokenQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn get_session_by_token_query(
    view: GetSessionByTokenQueryView,
    pool: PgPool,
) -> Result<Option<Session>, DatabaseError> {
    let result: Option<Session> = sqlx::query_as::<_, Session>(&view.get_request())
        .bind(view.get_token())
        .fetch_optional(&pool)
        .await?;

    Ok(result)
}
