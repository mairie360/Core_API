use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::rc::Rc;

// On suppose que vos fonctions existent ici
use api_lib::jwt_manager::{check_jwt_validity, get_jwt_from_request, JWTCheckError};

// 1. La structure de définition du Middleware
pub struct JwtMiddleware;

// 2. Implémentation de Transform (Factory)
// Cela permet d'initialiser le middleware dans App::new()
impl<S, B> Transform<S, ServiceRequest> for JwtMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

// 3. Le Service qui contient la logique
pub struct JwtMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for JwtMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();

        Box::pin(async move {
            // Extraction du JWT depuis les headers de la requête
            // Note: Adaptez `get_jwt_from_headers` pour qu'il prenne &HeaderMap ou &HttpRequest
            // Ici, on accède à req.request() pour avoir l'objet HttpRequest habituel
            let jwt_option = get_jwt_from_request(req.request());

            let jwt = match jwt_option {
                Some(token) => token,
                None => {
                    // Pas de token -> 401 Unauthorized immédiat
                    let response = HttpResponse::Unauthorized()
                        .body("Unauthorized: No JWT token provided.")
                        .map_into_right_body();
                    return Ok(req.into_response(response));
                }
            };

            // Vérification de validité (votre logique existante)
            match check_jwt_validity(&jwt).await {
                Ok(_) => {
                    // C'est valide ! On passe la main au handler suivant
                    let res = svc.call(req).await?;
                    // On map le body pour correspondre au type attendu (EitherBody)
                    Ok(res.map_into_left_body())
                }
                Err(error) => {
                    // Gestion des erreurs identique à votre macro
                    let response =
                        match error {
                            JWTCheckError::DatabaseError => HttpResponse::InternalServerError()
                                .body("Internal server error: Database not initialized."),
                            JWTCheckError::NoTokenProvided => HttpResponse::Unauthorized()
                                .body("Unauthorized: No JWT token provided."),
                            JWTCheckError::ExpiredToken => HttpResponse::Unauthorized()
                                .body("Unauthorized: JWT token is expired."),
                            JWTCheckError::InvalidToken => HttpResponse::Unauthorized()
                                .body("Unauthorized: Invalid JWT token."),
                            JWTCheckError::UnknowUser => {
                                HttpResponse::NotFound().body("User not found.")
                            }
                        };
                    Ok(req.into_response(response.map_into_right_body()))
                }
            }
        })
    }
}
