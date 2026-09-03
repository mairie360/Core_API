use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

use crate::database::roles::change_role::ChangeRoleQueryView;
use crate::database::roles::does_role_exist::{does_role_exist_query, DoesRoleExistQueryView};

pub async fn change_role_query(
    view: ChangeRoleQueryView,
    smart_db: &SmartDatabase,
) -> Result<(), ApiLibError> {
    // `execute()` ne remonte plus le nombre de lignes affectées : on vérifie donc
    // explicitement que le rôle existe avant d'appliquer la mise à jour, pour
    // conserver le comportement "erreur si mauvais ID".
    if !does_role_exist_query(DoesRoleExistQueryView::new(view.id()), smart_db).await? {
        return Err(ApiLibError::Database(DbError::NotFound));
    }

    smart_db.execute(view).await?;

    Ok(())
}
