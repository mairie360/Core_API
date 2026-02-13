use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/**
 * This macro is used to check the validity of a JWT token in an HTTP request.
 * It extracts the JWT from the request, checks its validity, and then executes the original function
 * logic if the JWT is valid. If the JWT is invalid or not present, it returns an appropriate HTTP response.
 *
 * Usage:
 * #[check_jwt]
 * async fn your_function(req: HttpRequest, path_view: web::Path<AboutPathParamRequestView>) -> HttpResponse {
 *     // Your function logic here
 * }
 *
 * # Returns:
 * - If the JWT is valid, it executes the original function logic.
 * - If the JWT is not present, it returns a 401 Unauthorized response.
 * - If the JWT is expired, it returns a 401 Unauthorized response.
 * - If the JWT is invalid, it returns a 401 Unauthorized response.
 * - If the JWT is associated with an unknown user, it returns a 404 Not Found response.
 * - If there is a database error, it returns a 500 Internal Server Error response
 */
#[proc_macro_attribute]
pub fn check_jwt(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let name = &input.sig.ident;
    let block = &input.block;

    let output = quote! {
        use api_lib::jwt_manager::check_jwt_validity;
        use api_lib::jwt_manager::get_jwt_from_request;
        use api_lib::jwt_manager::JWTCheckError;

        async fn #name(req: HttpRequest, path_view: web::Path<AboutPathParamRequestView>) -> HttpResponse {
            let jwt = match get_jwt_from_request(&req) {
                Some(token) => token,
                None => {
                    eprintln!("No JWT token found in the request.");
                    return HttpResponse::Unauthorized().body("Unauthorized: No JWT token provided.");
                }
            };

            match check_jwt_validity(&jwt).await {
                Ok(_) => {
                    // JWT is valid, proceed with the original function logic
                }
                Err(JWTCheckError::DatabaseError) => {
                    return HttpResponse::InternalServerError().body("Internal server error: Database not initialized.");
                }
                Err(JWTCheckError::NoTokenProvided) => {
                    return HttpResponse::Unauthorized().body("Unauthorized: No JWT token provided.");
                }
                Err(JWTCheckError::ExpiredToken) => {
                    return HttpResponse::Unauthorized().body("Unauthorized: JWT token is expired.");
                }
                Err(JWTCheckError::InvalidToken) => {
                    return HttpResponse::Unauthorized().body("Unauthorized: Invalid JWT token.");
                }
                Err(JWTCheckError::UnknowUser) => {
                    return HttpResponse::NotFound().body("User not found.");
                }
            }

            async fn original_logic(req: HttpRequest, path_view: web::Path<AboutPathParamRequestView>) -> HttpResponse {
                #block
            }

            original_logic(req, path_view).await
        }
    };

    output.into()
}
