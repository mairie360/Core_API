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

            // Appel de la fonction originale
            async fn original_logic(req: HttpRequest, path_view: web::Path<AboutPathParamRequestView>) -> HttpResponse {
                #block
            }

            original_logic(req, path_view).await
        }
    };

    output.into()
}
