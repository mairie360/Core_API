use actix_web::{middleware, web, App, HttpServer};

use core_api::auth_middleware::JwtMiddleware;
use core_api::endpoints::login::login_request::login;
use core_api::endpoints::register::register_request::register;
use core_api::endpoints::user::about::about_request::user_about;
use core_api::swagger::{health, hello, ApiDoc};

use mairie360_api_lib::database::db_interface::{get_db_interface, init_db_interface};
use mairie360_api_lib::env_manager::get_critical_env_var;
use mairie360_api_lib::redis::redis_manager::{create_redis_manager, get_redis_manager};

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

//                                        -- MAIN FUNCTION --

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_db_interface().await;
    match get_db_interface().lock().unwrap().as_mut() {
        Some(db_interface) => match db_interface.connect().await {
            Ok(msg) => {
                println!("{}", msg);
            }
            Err(e) => {
                eprintln!("Failed to connect to the database: {}", e);
                std::process::exit(1);
            }
        },
        None => {
            eprintln!("Database interface is not initialized.");
            std::process::exit(1);
        }
    }
    create_redis_manager().await;
    match get_redis_manager().await.as_mut() {
        Some(redis_manager) => match redis_manager.connect() {
            Ok(msg) => {
                println!("{}", msg);
            }
            Err(e) => {
                eprintln!("Failed to connect to Redis: {}", e);
                std::process::exit(1);
            }
        },
        None => {
            eprintln!("Redis manager is not initialized.");
            std::process::exit(1);
        }
    }
    let host = get_critical_env_var("HOST");
    let port = get_critical_env_var("PORT");
    let bind_address = format!("{}:{}", host, port);
    let server = HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            // post requests
            .service(hello)
            .service(register)
            .service(login)
            // get requests
            .service(health)
            .service(web::scope("").wrap(JwtMiddleware).service(user_about))
            // API documentation
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
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
