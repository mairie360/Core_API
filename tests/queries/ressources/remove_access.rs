use crate::common::get_pool;
use core_api::database::ressources::remove_access::{remove_access_query, RemoveAccessQueryView};
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, GROUP_OWNER_ID};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn success() {}

#[tokio::test]
#[serial]
async fn failure() {}
