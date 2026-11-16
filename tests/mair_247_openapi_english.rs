//! MAIR-247: the OpenAPI contract (what the BFFs read through `@mairie360/*-api-openapi`) is
//! written in English. Fails on any French text that comes back into a description, a summary
//! or an example.

use core_api::endpoints::swagger::ApiDoc;
use utoipa::OpenApi;

/// French texts kept on purpose, removed before the check.
const ALLOWED: &[&str] = &[
    // Texts mairie360_api_lib returns today (401 / 404 examples).
    "Jeton expiré",
    "Utilisateur inconnu",
    // Person name of an example.
    "Bensaïd",
];

const FRENCH_WORDS: &[&str] = &[
    "les",
    "des",
    "une",
    "est",
    "pour",
    "avec",
    "sans",
    "dans",
    "aucun",
    "aucune",
    "identifiant",
    "utilisateur",
    "requête",
    "erreur",
    "corps",
    "chemin",
    "renvoie",
];

fn strings(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(items) => items.iter().for_each(|v| strings(v, out)),
        serde_json::Value::Object(map) => map.values().for_each(|v| strings(v, out)),
        _ => {}
    }
}

fn is_french(text: &str) -> bool {
    let text = ALLOWED
        .iter()
        .fold(text.to_owned(), |t, a| t.replace(a, ""));
    text.chars().any(|c| "àâçéèêëîïôûùüÿœÀÂÇÉÈÊÎÔÛ".contains(c))
        || text
            .split(|c: char| !c.is_alphanumeric())
            .any(|word| FRENCH_WORDS.contains(&word.to_lowercase().as_str()))
}

#[test]
fn mair_247_openapi_contract_has_no_french_text() {
    let spec = serde_json::to_value(ApiDoc::openapi()).expect("the spec serializes");
    let mut texts = Vec::new();
    strings(&spec, &mut texts);
    let french: Vec<&String> = texts.iter().filter(|t| is_french(t)).collect();
    assert!(
        french.is_empty(),
        "French text in the OpenAPI contract: {french:#?}"
    );
}

#[test]
fn mair_247_detector_flags_french() {
    assert!(is_french("Identifiant de la conversation."));
    assert!(is_french("Erreur de base de données."));
    assert!(!is_french("Id of the conversation."));
    assert!(!is_french("Jeton expiré"));
}
