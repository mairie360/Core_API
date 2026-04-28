use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

use crate::database::queries::roles::change_role::ChangeRoleQueryView;

pub async fn change_role_query(
    view: ChangeRoleQueryView,
    pool: PgPool,
) -> Result<(), DatabaseError> {
    let rows_affected = sqlx::query(&view.get_request())
        .bind(view.name())
        .bind(view.description())
        .bind(view.can_be_deleted())
        .bind(view.id() as i64)
        .execute(&pool)
        .await?
        .rows_affected();

    if rows_affected == 0 {
        return Err(DatabaseError::Query(
            mairie360_api_lib::database::queries::QueryError::InvalidId("Bad ID".to_string()),
        ));
    }

    Ok(())
}
