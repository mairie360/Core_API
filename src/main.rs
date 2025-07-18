use actix_web::{get, post, App, HttpResponse, HttpServer, Responder};

use api_lib::database::db_interface::get_db_interface;
use core_api::endpoints::login::login_request::login;
use core_api::endpoints::login::login_view::LoginView;
use core_api::endpoints::register::register_request::register;
use core_api::endpoints::register::register_view::RegisterView;
use core_api::endpoints::user::about::about_request::user_about;
use core_api::get_critical_env_var;
use core_api::redis::redis_manager::{create_redis_manager, get_redis_manager};

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

//                                        -- POST REQUESTS --

/** * Handles a POST request to the root endpoint.
 * Responds with a simple "Hello, world!" message.
 */
#[utoipa::path(
    post,
    path = "/",
    responses(
        (status = 200, description = "Returns a greeting message", body = String)
    )
)]
#[post("/")]
pub async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

//                                        -- GET REQUESTS --

/** * Handles a GET request to the /health endpoint.
 * Responds with a simple "OK" message to indicate the service is healthy.
 */
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = String)
    )
)]
#[get("/health")]
pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[derive(OpenApi)]
#[openapi(
    paths(
        health,
        hello,
        core_api::endpoints::login::login_request::login,
        core_api::endpoints::register::register_request::register
    ),
    components(
        schemas(LoginView, RegisterView)
    ),
    tags(
        (name = "Core API", description = "Endpoints for core functionalities")
    )
)]
struct ApiDoc;

//                                        -- MAIN FUNCTION --

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
            // post requests
            .service(hello)
            .service(register)
            .service(login)
            // get requests
            .service(health)
            .service(user_about)
            // API documentation
            .service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", ApiDoc::openapi()))
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
