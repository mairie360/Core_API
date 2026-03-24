use crate::database::queries_result_views::AboutUserQueryResultView;
use crate::database::query_views::AboutUserQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use mairie360_api_lib::database::queries::QueryError;
use sqlx::PgPool;

pub async fn about_user_query(
    view: AboutUserQueryView,
    pool: PgPool,
) -> Result<AboutUserQueryResultView, DatabaseError> {
    let result = sqlx::query_as::<_, AboutUserQueryResultView>(&view.get_request())
        .bind(view.get_id() as i32)
        .fetch_optional(&pool)
        .await?;

    result.ok_or(DatabaseError::Query(QueryError::InvalidId(
        "User ID not found".to_string(),
    )))
}
