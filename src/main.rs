use actix_web::{
    App,
    HttpResponse,
    HttpServer,
    post,
    Responder,
};

use CoreAPI::register::register_request::register;
use CoreAPI::database::db_interface::get_db_interface;

//                                        -- POST REQUESTS --

#[post("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

//                                        -- MAIN FUNCTION --

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let server = HttpServer::new(
        || App::new()
        //post requests
        .service(hello)
        .service(register)
    )
    .bind("127.0.0.1:8080")?;

    let addr = server.addrs().first().cloned();
    tokio::spawn(async move {
        if let Some(addr) = addr {
            println!("Serveur démarré avec succès sur http://{}", addr);
        }
    });

    server.run().await
}
