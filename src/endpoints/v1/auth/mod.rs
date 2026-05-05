pub mod doc;
pub mod login;
pub mod register;

use actix_web::web;
use mairie360_api_lib::pool::AppState;

use crate::database::sessions::{
    create_session::{create_session_query, CreateSessionQueryView},
    revoke_previous_session::{revoke_previous_session_query, RevokePreviousSessionQueryView},
};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .service(login::endpoint::login)
            .service(register::endpoint::register),
    );
}

pub async fn revoke_previous_session(
    state: web::Data<AppState>,
    user_id: u64,
    ip_adress: &std::net::IpAddr,
    device_info: &str,
) {
    let view = RevokePreviousSessionQueryView::new(user_id, ip_adress.clone(), device_info);
    revoke_previous_session_query(view, state.db_pool.clone().unwrap())
        .await
        .map_err(|e| {
            eprintln!("Revoke Previous Session DB Error: {}", e);
        })
        .ok();
}

pub async fn create_new_session(
    state: web::Data<AppState>,
    user_id: u64,
    view: CreateSessionQueryView,
) {
    revoke_previous_session(
        state.clone(),
        user_id,
        view.get_ip_address(),
        view.get_device_info(),
    )
    .await;
    create_session_query(view, state.db_pool.clone().unwrap())
        .await
        .map_err(|e| {
            eprintln!("Create Session DB Error: {}", e);
        })
        .ok();
}
