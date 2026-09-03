use crate::database::ressources::get_ressource_type_id::GetRessourceTypeIdQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;

pub async fn get_ressource_type_id_query(
    view: GetRessourceTypeIdQueryView,
    smart_db: &SmartDatabase,
) -> Result<u64, ApiLibError> {
    let result: i32 = smart_db.fetch_scalar(&view).await?;

    Ok(result as u64)
}
