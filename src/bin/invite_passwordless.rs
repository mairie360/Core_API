//! One-shot job e-mailing the « Définir votre mot de passe » invitation to every non-archived
//! account that has no password, the accounts the Keycloak synchronisation created. Run it before
//! an instance switches to Core's own sign-in, then again to replay the failures.
//!
//! Reads the same `DB_*`, `REDIS_URL`, `RESEND_API_KEY`, `EMAIL_DOMAIN` and `PUBLIC_URL`
//! variables as the API. Without argument it only counts (dry run); `--send` sends the e-mails.
//! The report is printed as JSON on stdout (ids only, never an e-mail address) and the exit code
//! is `1` when an invitation failed.

// See lib.rs: several versions of transitive dependencies are outside our control.
#![allow(clippy::multiple_crate_versions)]

use core_api::emails::AccountEmails;
use core_api::invitations::invite_pending;
use mairie360_api_lib::env_manager::get_critical_env_var;
use mairie360_api_lib::state::AppState;
use std::process::ExitCode;

const SEND_FLAG: &str = "--send";

#[tokio::main]
async fn main() -> ExitCode {
    let mut dry_run = true;
    // Only the command-line flag is read here (the executable name is skipped), never a
    // security decision: `args` is safe for that.
    // nosemgrep: rust.lang.security.args.args
    for argument in std::env::args().skip(1) {
        if argument == SEND_FLAG {
            dry_run = false;
        } else {
            eprintln!("Unknown argument {argument}. Usage: invite_passwordless [{SEND_FLAG}]");
            return ExitCode::FAILURE;
        }
    }

    let emails = match AccountEmails::from_env() {
        Ok(emails) => emails,
        Err(e) => {
            eprintln!("The mailer is not configured: {e}");
            return ExitCode::FAILURE;
        }
    };
    let redis_url = get_critical_env_var("REDIS_URL");
    let db_user = get_critical_env_var("DB_USER");
    let db_password = get_critical_env_var("DB_PASSWORD");
    let db_host = get_critical_env_var("DB_HOST");
    let db_port = get_critical_env_var("DB_PORT");
    let db_name = get_critical_env_var("DB_NAME");
    let pg_url = format!("postgres://{db_user}:{db_password}@{db_host}:{db_port}/{db_name}");
    let state = AppState::new(redis_url, pg_url).await;

    match invite_pending(&state, &emails, dry_run).await {
        Ok(report) => {
            match serde_json::to_string_pretty(&report) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("Cannot serialise the report: {e}"),
            }
            if dry_run {
                eprintln!(
                    "Dry run: {} accounts without a password, nothing sent. Run with {SEND_FLAG}.",
                    report.total
                );
            } else {
                eprintln!(
                    "{} accounts without a password, {} invitations sent, {} failed.",
                    report.total,
                    report.sent,
                    report.failed_ids.len()
                );
            }
            if report.failed_ids.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("Invitation run aborted: {e}");
            ExitCode::FAILURE
        }
    }
}
