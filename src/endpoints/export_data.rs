//! Export of a user's data (MAIR-289): what `export_user_data()` of Devops/Database returns, as
//! is. Shared by `GET /api/v1/user/me/export` and `GET /api/v1/admin/users/{userId}/export`.
use crate::database::users::erasure::{ExportUserDataQueryView, UNKNOWN_USER};
use crate::endpoints::db_error;
use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::state::AppState;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    UnknownUser,
    DatabaseError,
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownUser => write!(f, "Unknown user"),
            Self::DatabaseError => write!(f, "An error occurred while accessing the database."),
        }
    }
}

impl ResponseError for ExportError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// The SQLSTATE of a database error raised by Postgres, if `error` is one.
#[must_use]
pub fn sqlstate(error: &ApiLibError) -> Option<String> {
    match error {
        ApiLibError::Database(DbError::Sqlx(sqlx::Error::Database(db_error))) => {
            db_error.code().map(std::borrow::Cow::into_owned)
        }
        _ => None,
    }
}

/// Every row the schema links to `user_id` (`export_user_data()`), credentials left out.
///
/// # Errors
///
/// [`ExportError::UnknownUser`] when no account has this id, [`ExportError::DatabaseError`]
/// otherwise.
pub async fn export_user_data(state: &AppState, user_id: u64) -> Result<Value, ExportError> {
    state
        .get_smart_db()
        .fetch_one::<Value, _>(&ExportUserDataQueryView::new(user_id))
        .await
        .map_err(|error| {
            if sqlstate(&error).as_deref() == Some(UNKNOWN_USER) {
                ExportError::UnknownUser
            } else {
                db_error::log("export user data", &error);
                ExportError::DatabaseError
            }
        })
}

/// Example of the export, for the `OpenAPI` document.
#[must_use]
pub fn example() -> Value {
    serde_json::json!({
        "user": { "id": 42, "first_name": "Jean", "last_name": "Dupont", "email": "jean.dupont@mairie360.fr", "status": "active", "is_archived": false },
        "data": {
            "group_members.user_id": [ { "group_id": 3, "user_id": 42 } ],
            "sessions.user_id": [ { "id": "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35", "user_id": 42, "device_info": "Chrome 140 on Windows 11" } ]
        },
        "exported_at": "2026-10-08T14:00:00+02:00"
    })
}
