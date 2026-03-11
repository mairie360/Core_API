use utoipa::OpenApi;
use core_api::swagger::ApiDoc; 

fn main() {
    // La méthode est maintenant visible
    let openapi_yaml = ApiDoc::openapi()
        .to_yaml()
        .expect("Failed to generate YAML");
    
    println!("{}", openapi_yaml);
}