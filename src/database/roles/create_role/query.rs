use crate::database::roles::create_role::view::CreateRoleQueryView;
use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use mairie360_api_lib::database::errors::DatabaseError;
use sqlx::PgPool;

pub async fn create_role_query(
    view: CreateRoleQueryView,
    pool: PgPool,
) -> Result<(), DatabaseError> {
    let request = view.get_request();
    let mut query = sqlx::query(&request)
        .bind(view.name())
        .bind(view.description());

    // 2. On n'ajoute le 3ème bind que si la valeur existe
    if let Some(val) = view.can_be_deleted() {
        query = query.bind(val);
    }

    query.execute(&pool).await?;
    Ok(())
}
