//! Database failures seen by the handlers (MAIR-421).
//!
//! A handler used to turn every database error into one status chosen by its author
//! (`map_err(|_| …)`): a constraint violation, an unreachable server and a bug all became the same
//! `404` or `400`, and nothing reached the logs. [`classify`] sorts an [`ApiLibError`] by what the
//! client can do about it, and [`log`] also writes the cause through `tracing` (into the request
//! span when the telemetry is on), so a `500` always has an exploitable cause in the logs.

use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;

/// SQLSTATE classes answered as a client error: invalid input value (`22xxx`, e.g. a value too
/// long for its column) and integrity constraint (`23xxx`, e.g. a `CHECK`).
const CLIENT_SQLSTATE_CLASSES: [&str; 2] = ["22", "23"];

/// What a database failure means for the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbFailure {
    /// A unique constraint refused the write: `409`.
    Conflict,
    /// No row matched, or a foreign key points to a row that does not exist: `404`.
    NotFound,
    /// Postgres refused a value of the request (data exception, check constraint): `400`.
    Invalid,
    /// Postgres cannot be reached right now: `503`.
    Unavailable,
    /// Anything else (mapping bug, Redis, unexpected SQL error): `500`.
    Internal,
}

/// Sorts `error` into a [`DbFailure`].
#[must_use]
pub fn classify(error: &ApiLibError) -> DbFailure {
    let ApiLibError::Database(error) = error else {
        return DbFailure::Internal;
    };
    match error {
        DbError::UniqueViolation(_) => DbFailure::Conflict,
        DbError::NotFound
        | DbError::ForeignKeyViolation(_)
        | DbError::Sqlx(sqlx::Error::RowNotFound) => DbFailure::NotFound,
        DbError::Sqlx(sqlx::Error::PoolClosed | sqlx::Error::PoolTimedOut | sqlx::Error::Io(_)) => {
            DbFailure::Unavailable
        }
        DbError::Sqlx(sqlx::Error::Database(db_error)) => match db_error.code() {
            Some(code) if CLIENT_SQLSTATE_CLASSES.iter().any(|c| code.starts_with(c)) => {
                DbFailure::Invalid
            }
            _ => DbFailure::Internal,
        },
        DbError::Internal(_) | DbError::MappingError(_) | DbError::Sqlx(_) => DbFailure::Internal,
    }
}

/// Logs `error` with `context` (what the handler was doing) and returns its [`DbFailure`].
///
/// Server-side failures are logged at `ERROR`, the ones caused by the request at `WARN`. The
/// message stays in the logs: callers answer with their own generic body, never with the
/// Postgres message (it names tables, columns and constraints).
pub fn log(context: &str, error: &ApiLibError) -> DbFailure {
    let failure = classify(error);
    match failure {
        DbFailure::Internal | DbFailure::Unavailable => {
            tracing::error!(context, ?failure, error = %error, "database error");
        }
        DbFailure::Conflict | DbFailure::NotFound | DbFailure::Invalid => {
            tracing::warn!(context, ?failure, error = %error, "database error");
        }
    }
    failure
}

#[cfg(test)]
mod tests {
    use super::{classify, DbFailure};
    use mairie360_api_lib::database::error::DbError;
    use mairie360_api_lib::error::ApiLibError;

    fn db(error: DbError) -> ApiLibError {
        ApiLibError::Database(error)
    }

    #[test]
    fn constraint_violations_are_client_errors() {
        assert_eq!(
            classify(&db(DbError::UniqueViolation("users_email_key".into()))),
            DbFailure::Conflict
        );
        assert_eq!(
            classify(&db(DbError::ForeignKeyViolation("fk".into()))),
            DbFailure::NotFound
        );
        assert_eq!(classify(&db(DbError::NotFound)), DbFailure::NotFound);
        assert_eq!(
            classify(&db(DbError::Sqlx(sqlx::Error::RowNotFound))),
            DbFailure::NotFound
        );
    }

    #[test]
    fn unreachable_database_is_unavailable() {
        assert_eq!(
            classify(&db(DbError::Sqlx(sqlx::Error::PoolTimedOut))),
            DbFailure::Unavailable
        );
        assert_eq!(
            classify(&db(DbError::Sqlx(sqlx::Error::PoolClosed))),
            DbFailure::Unavailable
        );
    }

    #[test]
    fn everything_else_is_internal() {
        assert_eq!(
            classify(&db(DbError::MappingError("bad json".into()))),
            DbFailure::Internal
        );
        assert_eq!(
            classify(&db(DbError::Internal("bind".into()))),
            DbFailure::Internal
        );
        assert_eq!(
            classify(&db(DbError::Sqlx(sqlx::Error::Protocol("x".into())))),
            DbFailure::Internal
        );
    }
}
