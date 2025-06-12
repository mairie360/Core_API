use actix_web::{
    App,
    get,
    HttpResponse,
    HttpServer,
    post,
    Responder,
};
use std::env;

use CoreAPI::register::register_request::register;
use CoreAPI::database::db_interface::get_db_interface;

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
    let host = match env::var("HOST") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("HOST environment variable not set.");
            std::process::exit(1);
        }
    };
    let port = match env::var("PORT") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("PORT environment variable not set");
            std::process::exit(1);
        },
    };
    let bind_address = format!("{}:{}", host, port);
    let server = HttpServer::new(
        || App::new()
        //post requests
        .service(hello)
        .service(register)

        //get requests
        .service(health)
    )
    .bind(bind_address)?;

    let addr = server.addrs().first().cloned();
    tokio::spawn(async move {
        if let Some(addr) = addr {
            println!("Serveur démarré avec succès sur http://{}", addr);
        }
    });

    server.run().await
}
