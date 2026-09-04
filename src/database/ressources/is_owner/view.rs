use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct IsOwnerQueryView {
    ressource_type: String,
    ressource_id: u64,
    owner_id: u64,
    params: Vec<QueryParam>,
}

impl IsOwnerQueryView {
    pub fn new(owner_id: u64, ressource_id: u64, ressource_type: &str) -> Self {
        Self {
            owner_id,
            ressource_id,
            ressource_type: ressource_type.to_string(),
            params: vec![
                QueryParam::I64(ressource_id as i64),
                QueryParam::I64(owner_id as i64),
            ],
        }
    }

    pub fn owner_id(&self) -> u64 {
        self.owner_id
    }

    pub fn ressource_id(&self) -> u64 {
        self.ressource_id
    }

    pub fn ressource_type(&self) -> &str {
        &self.ressource_type
    }
}

impl ApiRequestDto for IsOwnerQueryView {
    fn query_sql(&self) -> &'static str {
        // La table cible dépend de `ressource_type` (fourni par l'appelant), donc le texte de la
        // requête ne peut pas être un littéral statique unique : on le construit et on le "leak"
        // pour obtenir un &'static str, comme l'exige `ApiRequestDto`. Chaque appel fuit une petite
        // allocation ; à signaler pour une éventuelle évolution de la lib (SQL non statique).
        Box::leak(
            format!(
                "SELECT EXISTS(SELECT 1 FROM {} WHERE id = $1 AND owner_id = $2)",
                self.ressource_type
            )
            .into_boxed_str(),
        )
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for IsOwnerQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IsOwnerQueryView: owner_id = {}, ressource_id = {}, ressource_type = {}",
            self.owner_id, self.ressource_id, self.ressource_type
        )
    }
}
