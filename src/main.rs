use actix_web::{post, App, HttpResponse, HttpServer, Responder};

#[post("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let server = HttpServer::new(|| App::new().service(hello)).bind("127.0.0.1:8080")?;

    let addr = server.addrs().first().cloned();
    tokio::spawn(async move {
        if let Some(addr) = addr {
            println!("Serveur démarré avec succès sur http://{}", addr);
        }
    });

    server.run().await
}
