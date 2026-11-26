use crate::database::admin::list_users::AdminUserService;
use crate::database::ids::id_to_sql;
use crate::database::users::nullable_patch::push_text;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Reads the profile details of a user.
#[derive(Debug, Deserialize)]
pub struct GetProfileDetailsQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl GetProfileDetailsQueryView {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self {
            id,
            params: vec![QueryParam::I32(id_to_sql(id))],
        }
    }
}

impl ApiRequestDto for GetProfileDetailsQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT u.biography, u.job_title, u.photo IS NOT NULL AS has_photo, \
         (SELECT jsonb_build_object('id', sv.id, 'name', sv.name) FROM services sv WHERE sv.id = u.service_id) AS service \
         FROM users u WHERE u.id = $1) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetProfileDetailsQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetProfileDetailsQueryView: id = {}", self.id)
    }
}

/// Biography, job title, whether a photo is stored, and the service of a user.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileDetails {
    pub biography: Option<String>,
    pub job_title: Option<String>,
    pub has_photo: bool,
    pub service: Option<AdminUserService>,
}

/// Partial update of the biography and the job title. Outer `None` keeps the column, `Some(None)`
/// clears it, `Some(Some(v))` stores `v`.
#[derive(Debug, Deserialize)]
pub struct PatchProfileDetailsQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl PatchProfileDetailsQueryView {
    #[must_use]
    #[allow(clippy::option_option)]
    pub fn new(id: u64, biography: Option<Option<&str>>, job_title: Option<Option<&str>>) -> Self {
        let mut params = vec![QueryParam::I32(id_to_sql(id))];
        push_text(&mut params, biography);
        push_text(&mut params, job_title);
        Self { id, params }
    }
}

impl ApiRequestDto for PatchProfileDetailsQueryView {
    fn query_sql(&self) -> &'static str {
        "UPDATE users SET \
         biography = CASE WHEN $2 THEN CASE WHEN $3 THEN NULL ELSE $4 END ELSE biography END, \
         job_title = CASE WHEN $5 THEN CASE WHEN $6 THEN NULL ELSE $7 END ELSE job_title END \
         WHERE id = $1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for PatchProfileDetailsQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PatchProfileDetailsQueryView: id = {}", self.id)
    }
}

/// Stores the photo of a user. The bytes travel base64-encoded: the database layer binds no
/// `bytea` parameter, Postgres decodes them.
#[derive(Debug, Deserialize)]
pub struct SetPhotoQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl SetPhotoQueryView {
    #[must_use]
    pub fn new(id: u64, photo: &[u8]) -> Self {
        use base64::Engine;
        Self {
            id,
            params: vec![
                QueryParam::I32(id_to_sql(id)),
                QueryParam::Text(base64::engine::general_purpose::STANDARD.encode(photo)),
            ],
        }
    }
}

impl ApiRequestDto for SetPhotoQueryView {
    fn query_sql(&self) -> &'static str {
        "UPDATE users SET photo = decode($2, 'base64') WHERE id = $1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for SetPhotoQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SetPhotoQueryView: id = {}", self.id)
    }
}

/// Removes the photo of a user.
#[derive(Debug, Deserialize)]
pub struct ClearPhotoQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl ClearPhotoQueryView {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self {
            id,
            params: vec![QueryParam::I32(id_to_sql(id))],
        }
    }
}

impl ApiRequestDto for ClearPhotoQueryView {
    fn query_sql(&self) -> &'static str {
        "UPDATE users SET photo = NULL WHERE id = $1 AND photo IS NOT NULL"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for ClearPhotoQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClearPhotoQueryView: id = {}", self.id)
    }
}

/// Reads the photo of a user, base64-encoded (`null` when none).
#[derive(Debug, Deserialize)]
pub struct GetPhotoQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl GetPhotoQueryView {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self {
            id,
            params: vec![QueryParam::I32(id_to_sql(id))],
        }
    }
}

impl ApiRequestDto for GetPhotoQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT encode(photo, 'base64') AS photo FROM users WHERE id = $1) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetPhotoQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetPhotoQueryView: id = {}", self.id)
    }
}

/// Answer of [`GetPhotoQueryView`].
#[derive(Debug, Serialize, Deserialize)]
pub struct PhotoRow {
    photo: Option<String>,
}

impl PhotoRow {
    /// The stored bytes, `None` when the user has no photo. Postgres wraps base64 every 76
    /// characters, so the line breaks are dropped before decoding.
    #[must_use]
    pub fn bytes(&self) -> Option<Vec<u8>> {
        use base64::Engine;
        let encoded: String = self
            .photo
            .as_ref()?
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()
    }
}
