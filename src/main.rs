use actix_web::{get, post, App, HttpResponse, HttpServer, Responder};

use CoreAPI::database::db_interface::get_db_interface;
use CoreAPI::get_critical_env_var;
use CoreAPI::register::register_request::register;

//                                        -- POST REQUESTS --

#[post("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

//                                        -- GET REQUESTS --

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

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
    let host = get_critical_env_var("HOST");
    let port = get_critical_env_var("PORT");
    let bind_address = format!("{}:{}", host, port);
    let server = HttpServer::new(|| {
        App::new()
            // post requests
            .service(hello)
            .service(register)
            // get requests
            .service(health)
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
