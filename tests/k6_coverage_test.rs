//! `load-test.js` must hold exactly one handler per operation of the published contract: the k6
//! `OpenAPI` coverage gate (mairie360/CICD `coverage.js`) aborts the performance stack otherwise, and
//! that stack only runs after the merge (nightly gates, staging promotion). This test catches the
//! drift in `cargo test`, before the merge.
use std::collections::BTreeSet;

use core_api::endpoints::swagger::ApiDoc;
use utoipa::OpenApi;

const METHODS: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];

/// The `'METHOD /path'` keys of the handler maps of `load-test.js`.
fn load_test_handlers() -> BTreeSet<String> {
    let script = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/load-test.js"))
        .expect("load-test.js is readable");
    script
        .lines()
        .filter_map(|line| {
            let key = line.trim_start().strip_prefix('\'')?;
            let (key, rest) = key.split_once('\'')?;
            let method = key.split(' ').next()?;
            (rest.starts_with(':') && METHODS.contains(&method)).then(|| key.to_string())
        })
        .collect()
}

/// The `METHOD /path` of every operation of the contract.
fn contract_operations() -> BTreeSet<String> {
    let document = serde_json::to_value(ApiDoc::openapi()).expect("serialisable contract");
    document["paths"]
        .as_object()
        .expect("paths")
        .iter()
        .flat_map(|(path, operations)| {
            operations
                .as_object()
                .expect("operations")
                .keys()
                .map(move |method| format!("{} {path}", method.to_uppercase()))
        })
        .filter(|operation| METHODS.iter().any(|m| operation.starts_with(m)))
        .collect()
}

#[test]
fn mair_812_load_test_handlers_match_the_contract() {
    let handlers = load_test_handlers();
    let operations = contract_operations();
    let stale: Vec<_> = handlers.difference(&operations).collect();
    let missing: Vec<_> = operations.difference(&handlers).collect();
    assert!(
        stale.is_empty() && missing.is_empty(),
        "load-test.js is out of sync with the contract.\n\
         Handlers of no operation (removed routes): {stale:?}\n\
         Operations without a handler: {missing:?}"
    );
}
