use actix_web::{middleware, web, App, HttpServer};

use core_api::endpoints::config;
use core_api::endpoints::swagger::ApiDoc;
use core_api::endpoints::{health, hello};
use mairie360_api_lib::security::JwtMiddleware;

use mairie360_api_lib::env_manager::get_critical_env_var;
use mairie360_api_lib::pool::AppState;
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
    let pg_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        db_user, db_password, db_host, db_port, db_name
    );
    let state = AppState::new(redis_url, pg_url).await;
    let data = web::Data::new(state);
    let host = get_critical_env_var("HOST");
    let port = get_critical_env_var("PORT");
    let bind_address = format!("{}:{}", host, port);
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
            // 3. Endpoints Protégés par JWT
            .service(
                web::scope("/api").wrap(JwtMiddleware).configure(config), // Tes routes v1, etc.
            )
    })
    .bind(bind_address)?;

    let addr = server.addrs().first().copied();
    tokio::spawn(async move {
        if let Some(addr) = addr {
            println!("Serveur démarré avec succès sur http://{}", addr);
        }
    });

    server.run().await
}
