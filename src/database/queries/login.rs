use crate::database::queries_result_views::LoginUserQueryResultView;
use crate::database::query_views::LoginUserQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn login_query(
    view: LoginUserQueryView,
    pool: PgPool,
) -> Result<Option<LoginUserQueryResultView>, DatabaseError> {
    let result = sqlx::query_as::<_, LoginUserQueryResultView>(&view.get_request())
        .bind(view.get_email())
        .fetch_optional(&pool)
        .await?;

    Ok(result)
}
