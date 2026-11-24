//! MAIR-776: the public role list (`GET /api/v1/roles/`) and the administration's (`GET /api/v1/admin/roles/`)
//! both had a schema named `Role`. utoipa keeps one schema per name, so the public route was documented with the
//! administration's fields (`can_be_deleted`, `modules`) it never returns, and every consumer validating its
//! answer against the contract refused it.

use core_api::endpoints::swagger::ApiDoc;
use serde_json::Value;
use utoipa::OpenApi;

/// Properties of the role items of the 200 answer of `GET <path>`, references resolved.
fn role_item_properties(path: &str) -> Vec<String> {
    let document = serde_json::to_value(ApiDoc::openapi()).unwrap();
    let resolve = |schema: &Value| -> Value {
        schema["$ref"].as_str().map_or_else(
            || schema.clone(),
            |reference| {
                let name = reference.rsplit('/').next().unwrap();
                document["components"]["schemas"][name].clone()
            },
        )
    };
    let answer = resolve(
        &document["paths"][path]["get"]["responses"]["200"]["content"]["application/json"]
            ["schema"],
    );
    let roles = resolve(&answer["properties"]["roles"]);
    let item = resolve(&roles["items"]);
    let mut properties: Vec<String> = item["properties"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    properties.sort();
    properties
}

#[test]
fn mair_776_the_public_role_list_documents_only_the_fields_it_returns() {
    assert_eq!(
        role_item_properties("/api/v1/roles/"),
        ["description", "id", "name"]
    );
}

#[test]
fn mair_776_the_admin_role_list_keeps_its_own_role_schema() {
    let properties = role_item_properties("/api/v1/admin/roles/");
    assert!(
        properties.contains(&"can_be_deleted".to_string()),
        "{properties:?}"
    );
    assert!(
        properties.contains(&"modules".to_string()),
        "{properties:?}"
    );
}
