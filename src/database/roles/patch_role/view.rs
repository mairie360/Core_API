use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(Debug)]
pub struct PatchRoleQueryView {
    id: u64,
    name: Option<String>,
    description: Option<String>,
    can_be_deleted: Option<Option<bool>>,
    // `query_sql` doit renvoyer un &'static str, mais les colonnes patchées (donc le texte SQL)
    // varient à chaque appel selon les champs fournis. On construit la requête une fois dans
    // `new()` et on la "leak" pour obtenir un &'static str, comme le fait déjà cette lib pour les
    // cas de SQL dynamique (voir IsOwnerQueryView). Chaque appel fuit une petite allocation.
    sql: &'static str,
    params: Vec<QueryParam>,
}

impl PatchRoleQueryView {
    pub fn new(
        id: u64,
        name: Option<String>,
        description: Option<String>,
        can_be_deleted: Option<Option<bool>>,
    ) -> Self {
        let mut set_clauses: Vec<String> = Vec::new();
        let mut params: Vec<QueryParam> = Vec::new();

        if let Some(name) = &name {
            params.push(QueryParam::Text(name.clone()));
            set_clauses.push(format!("name = ${}", params.len()));
        }
        if let Some(description) = &description {
            params.push(QueryParam::Text(description.clone()));
            set_clauses.push(format!("description = ${}", params.len()));
        }
        match can_be_deleted {
            Some(Some(value)) => {
                params.push(QueryParam::Bool(value));
                set_clauses.push(format!("can_be_deleted = ${}", params.len()));
            }
            // Le champ est fourni mais explicitement à null : on le met à NULL en dur (mot-clé
            // fixe, aucune donnée appelante interpolée) plutôt que de tenter de bind un NULL, que
            // `QueryParam` ne sait pas représenter pour un bool.
            Some(None) => set_clauses.push("can_be_deleted = NULL".to_string()),
            None => {}
        }

        let sql: &'static str = if set_clauses.is_empty() {
            ""
        } else {
            params.push(QueryParam::I64(id as i64));
            Box::leak(
                format!(
                    "UPDATE roles SET {} WHERE id = ${}",
                    set_clauses.join(", "),
                    params.len()
                )
                .into_boxed_str(),
            )
        };

        Self {
            id,
            name,
            description,
            can_be_deleted,
            sql,
            params,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn can_be_deleted(&self) -> Option<Option<bool>> {
        self.can_be_deleted
    }

    /// Vrai si aucun champ n'a été fourni : il n'y a alors rien à écrire en base.
    pub fn is_noop(&self) -> bool {
        self.sql.is_empty()
    }
}

// Impl manuelle : un champ `&'static str` empêche `#[derive(Deserialize)]` de produire un
// `impl<'de> Deserialize<'de>` valide pour *toute* durée de vie 'de (requis par `DeserializeOwned`
// via `ApiRequestDto`). Cette vue n'est jamais réellement désérialisée, donc l'impl n'a pas besoin
// de faire mieux qu'échouer proprement si elle l'était.
impl<'de> serde::Deserialize<'de> for PatchRoleQueryView {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "PatchRoleQueryView is not deserializable",
        ))
    }
}

impl ApiRequestDto for PatchRoleQueryView {
    fn query_sql(&self) -> &'static str {
        self.sql
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for PatchRoleQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PatchRoleQueryView: id = {}, name = {:?}, description = {:?}, can_be_deleted = {:?}",
            self.id, self.name, self.description, self.can_be_deleted,
        )
    }
}
