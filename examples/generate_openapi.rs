use core_api::endpoints::swagger::ApiDoc;
use utoipa::OpenApi;

fn main() {
    // 1. Obtenir la structure OpenApi
    let doc = ApiDoc::openapi();

    // 2. Utiliser serde_yaml ou la méthode intégrée si la feature "yaml" est activée
    // Si vous avez la feature "yaml" dans Cargo.toml :
    match doc.to_yaml() {
        Ok(yaml) => println!("{}", yaml),
        Err(err) => eprintln!("Erreur lors de la génération du YAML : {}", err),
    }
}
