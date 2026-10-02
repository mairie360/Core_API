// `clippy::assert_is_empty` (new in Rust 1.99) would turn `assert!(list.is_empty())` into
// `assert_eq!(list, [] as [T; 0])` with the full item type: less readable in tests.
#![allow(clippy::assert_is_empty)]
// Fixtures cast seeded Postgres ids (small positive values) between u64 and i32 / i64; the API
// code itself converts through `database::ids` (MAIR-422).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

mod common; // Accès à ton pool
mod endpoints;
mod keycloak;
mod queries;
