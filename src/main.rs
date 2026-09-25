// Voir lib.rs : versions multiples de dépendances transitives hors de notre contrôle.
#![allow(clippy::multiple_crate_versions)]

use actix_web::{middleware, web, App, HttpServer};

use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::swagger::ApiDoc;
use core_api::endpoints::{config, public_config};
use core_api::endpoints::{health, hello};
use mairie360_api_lib::security::JwtMiddleware;

use mairie360_api_lib::env_manager::get_critical_env_var;
use mairie360_api_lib::state::AppState;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

//                                        -- MAIN FUNCTION --

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let redis_url = get_critical_env_var("REDIS_URL");
    let db_user = get_critical_env_var("DB_USER");
    let db_password = get_critical_env_var("DB_PASSWORD");
    let db_host = get_critical_env_var("DB_HOST");
    let db_port = get_critical_env_var("DB_PORT");
    let db_name = get_critical_env_var("DB_NAME");
    let pg_url = format!("postgres://{db_user}:{db_password}@{db_host}:{db_port}/{db_name}");
    let state = AppState::new(redis_url, pg_url).await;
    let data = web::Data::new(state);
    let host = get_critical_env_var("HOST");
    let port = get_critical_env_var("PORT");
    let bind_address = format!("{host}:{port}");
    let server = HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(middleware::Logger::default())
            // post requests
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
            .service(health::health)
            .service(hello::hello)
            // Routes /api publiques (refresh du JWT) : avant le scope protégé, qui sinon les capte
            .configure(public_config)
            // 3. Endpoints Protégés par JWT
            // The last `wrap` runs first: JwtMiddleware validates the JWT, then session_guard
            // rejects it if its session was revoked (MAIR-226).
            .service(
                web::scope("/api")
                    .wrap(middleware::from_fn(session_guard))
                    .wrap(JwtMiddleware)
                    .configure(config),
            )
    })
    .bind(bind_address)?;

    let addr = server.addrs().first().copied();
    tokio::spawn(async move {
        if let Some(addr) = addr {
            println!("Serveur démarré avec succès sur http://{addr}");
        }
    });

    server.run().await
}
