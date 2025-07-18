use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn check_jwt(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let name = &input.sig.ident;
    let block = &input.block;

    let output = quote! {
        async fn #name(req: HttpRequest, path_view: web::Path<AboutPathParamRequestView>) -> HttpResponse {
            let jwt = match get_jwt_from_request(&req) {
                Some(token) => token,
                None => {
                    eprintln!("No JWT token found in the request.");
                    return HttpResponse::Unauthorized().body("Unauthorized: No JWT token provided.");
                }
            };

            println!("JWT token: {}", jwt);
            let user_id = match get_user_id_from_jwt(&jwt) {
                Some(id) => id,
                None => {
                    eprintln!("Failed to decode JWT token.");
                    return HttpResponse::Unauthorized().body("Unauthorized: Invalid JWT token.");
                }
            };
            println!("User ID from JWT: {}", user_id);

            let view = DoesUserExistByIdQueryView::new(user_id.parse().unwrap());
            let db_guard = get_db_interface().lock().unwrap();
            let db_interface = match &*db_guard {
                Some(db) => db,
                None => {
                    eprintln!("Database interface is not initialized.");
                    return HttpResponse::InternalServerError().body("Internal server error: Database not initialized.");
                }
            };
            let query_view = db_interface.execute_query(Box::new(view)).await;

            let result = match query_view {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("Error executing query: {}", e);
                    return HttpResponse::InternalServerError().body("Internal server error: Database query failed.");
                }
            };

            if !get_boolean_from_query_result(result.get_result()) {
                eprintln!("User does not exist with ID: {}", user_id);
                return HttpResponse::NotFound().body("User not found.")
            }

            let timeout: usize = match get_timeout_from_jwt(&jwt) {
                Some(t) => t,
                None => {
                    eprintln!("Failed to retrieve timeout from JWT.");
                    return HttpResponse::Unauthorized().body("Unauthorized: Invalid JWT token.");
                }
            };

            match verify_jwt_timeout(timeout) {
                Ok(true) => {
                    println!("JWT token is valid and not expired.");
                },
                Ok(false) => {
                    eprintln!("JWT token is expired or invalid.");
                    return HttpResponse::Unauthorized().body("Unauthorized: JWT token is expired or invalid.");
                },
                Err(e) => {
                    eprintln!("Error verifying JWT timeout: {}", e);
                    return HttpResponse::InternalServerError().body("Internal server error: Failed to verify JWT timeout.");
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
