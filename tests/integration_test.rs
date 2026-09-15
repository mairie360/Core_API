// Voir src/lib.rs : casts u64<->i32/i64 sûrs par construction sur des ids Postgres.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

mod common; // Accès à ton pool
mod queries;
