//! One-shot job migrating the Mairie 360 accounts and roles to Keycloak (MAIR-141), the
//! command-line twin of `POST /api/v1/admin/keycloak/migration` for Kubernetes jobs and
//! operators.
//!
//! Reads the same `DB_*`, `REDIS_URL` and `KEYCLOAK_*` variables as the API, prints the run
//! report as JSON on stdout and exits with `1` when the run could not complete or at least one
//! account failed, so a replay is scheduled. `--send-password-setup-email` asks Keycloak to
//! e-mail a password set-up link to every account created by this run.

// See lib.rs: several versions of transitive dependencies are outside our control.
#![allow(clippy::multiple_crate_versions)]

use core_api::keycloak::migration::{migrate_users, MigrationOptions};
use core_api::keycloak::{KeycloakAdminClient, KeycloakConfig};
use mairie360_api_lib::env_manager::get_critical_env_var;
use mairie360_api_lib::state::AppState;
use std::process::ExitCode;

const SEND_PASSWORD_SETUP_EMAIL_FLAG: &str = "--send-password-setup-email";

#[tokio::main]
async fn main() -> ExitCode {
    let mut options = MigrationOptions::default();
    // Only the command-line flag is read here (the executable name is skipped), never a
    // security decision: `args` is safe for that.
    // nosemgrep: rust.lang.security.args.args
    for argument in std::env::args().skip(1) {
        if argument == SEND_PASSWORD_SETUP_EMAIL_FLAG {
            options.send_password_setup_email = true;
        } else {
            eprintln!("Unknown argument {argument}. Usage: keycloak_migration [{SEND_PASSWORD_SETUP_EMAIL_FLAG}]");
            return ExitCode::FAILURE;
        }
    }

    let Some(config) = KeycloakConfig::from_env() else {
        eprintln!("Keycloak is not configured: set KEYCLOAK_REALM_URL, KEYCLOAK_CLIENT_ID and KEYCLOAK_CLIENT_SECRET.");
        return ExitCode::FAILURE;
    };
    let admin = KeycloakAdminClient::new(config);

    let redis_url = get_critical_env_var("REDIS_URL");
    let db_user = get_critical_env_var("DB_USER");
    let db_password = get_critical_env_var("DB_PASSWORD");
    let db_host = get_critical_env_var("DB_HOST");
    let db_port = get_critical_env_var("DB_PORT");
    let db_name = get_critical_env_var("DB_NAME");
    let pg_url = format!("postgres://{db_user}:{db_password}@{db_host}:{db_port}/{db_name}");
    let state = AppState::new(redis_url, pg_url).await;

    match migrate_users(state.get_smart_db(), &admin, options).await {
        Ok(report) => {
            match serde_json::to_string_pretty(&report) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("Cannot serialise the migration report: {e}"),
            }
            eprintln!(
                "Keycloak migration: {} accounts, {} created, {} updated, {} disabled, {} failed.",
                report.total, report.created, report.updated, report.disabled, report.failed
            );
            if report.failed == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("Keycloak migration aborted: {error}");
            ExitCode::FAILURE
        }
    }
}
