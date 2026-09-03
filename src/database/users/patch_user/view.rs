use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(Debug)]
pub struct PatchUserQueryView {
    id: u64,
    first_name: Option<String>,
    last_name: Option<String>,
    email: Option<String>,
    phone_number: Option<String>,
    password: Option<String>,
    // Cf. PatchRoleQueryView : le nombre/l'ordre des colonnes patchées varie selon les champs
    // fournis, donc le texte SQL ne peut pas être un unique littéral statique. On le construit une
    // fois dans `new()` (en ne liant que les valeurs réellement fournies, via des paramètres
    // positionnés — donc sans l'interpolation de chaînes non paramétrée qu'utilisait l'ancienne
    // implémentation de `get_request()`) et on le "leak" pour obtenir un &'static str.
    sql: &'static str,
    params: Vec<QueryParam>,
}

impl PatchUserQueryView {
    pub fn new(
        id: u64,
        first_name: Option<&str>,
        last_name: Option<&str>,
        email: Option<&str>,
        phone_number: Option<&str>,
        password: Option<&str>,
    ) -> Self {
        let mut set_clauses: Vec<String> = Vec::new();
        let mut params: Vec<QueryParam> = Vec::new();

        if let Some(first_name) = first_name {
            params.push(QueryParam::Text(first_name.to_string()));
            set_clauses.push(format!("first_name = ${}", params.len()));
        }
        if let Some(last_name) = last_name {
            params.push(QueryParam::Text(last_name.to_string()));
            set_clauses.push(format!("last_name = ${}", params.len()));
        }
        if let Some(email) = email {
            params.push(QueryParam::Text(email.to_string()));
            set_clauses.push(format!("email = ${}", params.len()));
        }
        if let Some(phone_number) = phone_number {
            params.push(QueryParam::Text(phone_number.to_string()));
            set_clauses.push(format!("phone_number = ${}", params.len()));
        }
        if let Some(password) = password {
            params.push(QueryParam::Text(password.to_string()));
            set_clauses.push(format!("password = ${}", params.len()));
        }

        let sql: &'static str = if set_clauses.is_empty() {
            ""
        } else {
            params.push(QueryParam::I32(id as i32));
            Box::leak(
                format!(
                    "UPDATE users SET {} WHERE id = ${}",
                    set_clauses.join(", "),
                    params.len()
                )
                .into_boxed_str(),
            )
        };

        Self {
            id,
            first_name: first_name.map(|s| s.to_string()),
            last_name: last_name.map(|s| s.to_string()),
            email: email.map(|s| s.to_string()),
            phone_number: phone_number.map(|s| s.to_string()),
            password: password.map(|s| s.to_string()),
            sql,
            params,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn first_name(&self) -> Option<&str> {
        self.first_name.as_deref()
    }
    pub fn last_name(&self) -> Option<&str> {
        self.last_name.as_deref()
    }
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }
    pub fn phone_number(&self) -> Option<&str> {
        self.phone_number.as_deref()
    }
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// Vrai si aucun champ n'a été fourni : il n'y a alors rien à écrire en base.
    pub fn is_noop(&self) -> bool {
        self.sql.is_empty()
    }
}

// Cf. PatchRoleQueryView : impl manuelle nécessaire à cause du champ `&'static str`.
impl<'de> serde::Deserialize<'de> for PatchUserQueryView {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "PatchUserQueryView is not deserializable",
        ))
    }
}

impl ApiRequestDto for PatchUserQueryView {
    fn query_sql(&self) -> &'static str {
        self.sql
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for PatchUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.phone_number {
            Some(_) => write!(f, "PatchUserQueryView: id = {:?}, first_name = {:?}, last_name = {:?}, email = {:?}, phone_number = {:?}, password = {:?}", self.id(), self.first_name(), self.last_name(), self.email(), self.phone_number(), self.password()),
            None => write!(f, "PatchUserQueryView: id = {:?}, first_name = {:?}, last_name = {:?}, email = {:?}, password = {:?}", self.id(), self.first_name(), self.last_name(), self.email(), self.password()),
        }
    }
}
