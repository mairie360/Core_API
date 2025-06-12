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
use CoreAPI::database::db_interface::db_interface;
use CoreAPI::get_critical_env_var;

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
    let mut interface = db_interface::new();
    interface.connect().await;
    let host = get_critical_env_var("HOST");
    let port = get_critical_env_var("PORT");
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
