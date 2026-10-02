//! Conversions between API ids (`u64`) and the `INT4` / `INT8` ids of Postgres (MAIR-422).
//!
//! An `as` cast wraps: `2^32 + 2 as i32` is `2`, so `/user/4294967298/` used to read user 2.
//! These conversions saturate instead: an id beyond the column's range becomes its maximum,
//! which no row has, so the request answers like any unknown id (`404`).

pub use mairie360_api_lib::database::db_interface::{id_from_sql, id_to_sql};

/// `id` as an `INT8` parameter, saturated at `i64::MAX`.
#[must_use]
pub fn id_to_sql_i64(id: u64) -> i64 {
    i64::try_from(id).unwrap_or(i64::MAX)
}

/// An `INT8` id read from Postgres as an API id; a negative value gives `0`.
#[must_use]
pub fn id_from_sql_i64(id: i64) -> u64 {
    u64::try_from(id).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{id_from_sql, id_from_sql_i64, id_to_sql, id_to_sql_i64};

    #[test]
    fn ids_beyond_the_column_saturate_instead_of_wrapping() {
        assert_eq!(id_to_sql(2), 2);
        assert_eq!(id_to_sql(4_294_967_298), i32::MAX);
        assert_eq!(id_to_sql(u64::MAX), i32::MAX);
        assert_eq!(id_to_sql_i64(u64::MAX), i64::MAX);
        assert_eq!(id_to_sql_i64(42), 42);
    }

    #[test]
    fn negative_sql_ids_become_zero() {
        assert_eq!(id_from_sql(-1), 0);
        assert_eq!(id_from_sql(7), 7);
        assert_eq!(id_from_sql_i64(-1), 0);
        assert_eq!(id_from_sql_i64(7), 7);
    }
}
