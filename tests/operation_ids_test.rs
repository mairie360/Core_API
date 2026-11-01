//! MAIR-692: the generated clients (`@mairie360/core-api-openapi`) name their functions after the
//! `operationId`, which utoipa takes from the handler name. Two handlers named `logout` made orval
//! rename `POST /api/v1/sessions/logout` to `logout2` and give `logout` to the new route, so a
//! consumer calling `logout()` silently hit another operation.

use core_api::endpoints::swagger::ApiDoc;
use std::collections::HashMap;
use utoipa::OpenApi;

/// (operationId as orval names it, "METHOD path") of every operation of the contract.
fn operations() -> Vec<(String, String)> {
    let document = serde_json::to_value(ApiDoc::openapi()).unwrap();
    let mut operations = Vec::new();
    for (path, item) in document["paths"].as_object().unwrap() {
        for (method, operation) in item.as_object().unwrap() {
            let Some(id) = operation["operationId"].as_str() else {
                continue;
            };
            // orval camel-cases the id: `logout_with_refresh_token` and `logoutWithRefreshToken` clash.
            let key = id.replace('_', "").to_lowercase();
            operations.push((key, format!("{} {path}", method.to_uppercase())));
        }
    }
    operations
}

#[test]
fn mair_692_every_operation_has_its_own_client_name() {
    let mut seen: HashMap<String, String> = HashMap::new();
    for (key, operation) in operations() {
        if let Some(first) = seen.insert(key.clone(), operation.clone()) {
            panic!("`{key}` names both {first} and {operation}: the generated client renames one");
        }
    }
}

#[test]
fn mair_692_existing_logout_keeps_its_client_name() {
    let by_operation: HashMap<String, String> = operations()
        .into_iter()
        .map(|(key, op)| (op, key))
        .collect();
    assert_eq!(by_operation["POST /api/v1/sessions/logout"], "logout");
    assert_eq!(
        by_operation["POST /api/v1/auth/logout"],
        "logoutwithrefreshtoken"
    );
}
