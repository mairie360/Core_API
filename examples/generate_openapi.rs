use core_api::swagger::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let openapi_yaml = ApiDoc::openapi()
        .to_yaml()
        .expect("Failed to generate YAML");

    println!("{}", openapi_yaml);
}
