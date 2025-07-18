use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn check_jwt(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let name = &input.sig.ident;
    let block = &input.block;

    let output = quote! {
        use api_lib::jwt_manager::check_jwt_validity;
        use api_lib::jwt_manager::JWTCheckError;

        async fn #name(req: HttpRequest, path_view: web::Path<AboutPathParamRequestView>) -> HttpResponse {
            let jwt = match get_jwt_from_request(&req) {
                Some(token) => token,
                None => {
                    eprintln!("No JWT token found in the request.");
                    return HttpResponse::Unauthorized().body("Unauthorized: No JWT token provided.");
                }
            };

            println!("JWT token: {}", jwt);
            match check_jwt_validity(&jwt).await {
                Ok(_) => println!("JWT token is valid."),
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
