use actix_web::{http::Method, test, web, App, HttpResponse};
use core_api::endpoints::swagger::ApiDoc;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::state::AppState;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use utoipa::OpenApi;

// Chaque opération publiée dans le contrat OpenAPI (celui dont est généré @mairie360/core-api-openapi) doit
// correspondre à une route actix réellement montée (Core ne normalise pas le slash final). Aucune base ni JWT
// n'est nécessaire : une route absente tombe sur le service par défaut (418), une route trouvée échoue plus loin
// (JWT, droits, corps). Limite : AdminMiddleware répond 401 sans JWT avant le routage, les chemins /admin/ ne
// sont donc vérifiés qu'en l'absence de 418.
#[actix_web::test]
async fn every_published_operation_is_routed() {
    // AdminMiddleware exige un AppState. Sans base joignable, la lib retente la connexion pendant 30 s :
    // on réutilise la base de test partagée (Redis absent, ses échecs sont silencieux).
    let (_container, pg_url) = get_shared_db().await;
    let state = AppState::new("redis://127.0.0.1:6379".to_string(), pg_url.to_string()).await;
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            // Routes /api publiques (refresh du JWT) : montées hors du scope protégé, comme dans main.rs.
            .configure(public_config)
            .service(web::scope("/api").configure(config))
            .default_service(web::to(HttpResponse::ImATeapot)),
    )
    .await;

    let document = serde_json::to_value(ApiDoc::openapi()).expect("contrat OpenAPI sérialisable");
    let paths = document["paths"].as_object().expect("paths");
    let mut unrouted = Vec::new();

    for (template, operations) in paths
        .iter()
        .filter(|(path, _)| path.starts_with("/api/v1/"))
    {
        if template.contains("//") {
            unrouted.push(format!("segment vide dans {template}"));
        }
        let uri = template
            .split('/')
            .map(|segment| {
                if segment.starts_with('{') {
                    "1"
                } else {
                    segment
                }
            })
            .collect::<Vec<_>>()
            .join("/");

        for method in operations.as_object().expect("opérations").keys() {
            let method =
                Method::from_bytes(method.to_uppercase().as_bytes()).expect("méthode HTTP");
            let request = test::TestRequest::default()
                .method(method.clone())
                .uri(&uri)
                .to_request();
            // Les middlewares (JWT, admin, droits) renvoient une erreur au lieu d'une réponse.
            let status = match test::try_call_service(&app, request).await {
                Ok(response) => response.status(),
                Err(error) => error.as_response_error().status_code(),
            };
            if status.as_u16() == 418 {
                unrouted.push(format!("{method} {template}"));
            }
        }
    }

    assert!(
        unrouted.is_empty(),
        "opérations publiées sans route actix : {unrouted:?}"
    );
}
