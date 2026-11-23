//! `load-test.js` must hold exactly one handler per operation of the published contract: the k6
//! `OpenAPI` coverage gate (mairie360/CICD `coverage.js`) aborts the performance stack otherwise, and
//! that stack only runs after the merge (nightly gates, staging promotion). This test catches the
//! drift in `cargo test`, before the merge.
use std::collections::BTreeSet;

use core_api::endpoints::swagger::ApiDoc;
use utoipa::OpenApi;

const METHODS: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];

/// The `'METHOD /path'` (or double-quoted) keys of the handler maps of `load-test.js`, in file
/// order (a key written twice appears twice: JavaScript silently keeps the last one).
fn load_test_handler_keys() -> Vec<String> {
    let script = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/load-test.js"))
        .expect("load-test.js is readable");
    script
        .lines()
        .filter_map(|line| {
            // `'METHOD /path'` or `"METHOD /path"`: JavaScript treats both as the same key.
            let line = line.trim_start();
            let quote = line.chars().next().filter(|c| matches!(c, '\'' | '"'))?;
            let (key, rest) = line[1..].split_once(quote)?;
            let method = key.split(' ').next()?;
            (rest.starts_with(':') && METHODS.contains(&method)).then(|| key.to_string())
        })
        .collect()
}

fn load_test_handlers() -> BTreeSet<String> {
    load_test_handler_keys().into_iter().collect()
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

#[test]
fn mair_812_load_test_has_one_handler_per_operation() {
    let mut seen = BTreeSet::new();
    let duplicated: BTreeSet<_> = load_test_handler_keys()
        .into_iter()
        .filter(|key| !seen.insert(key.clone()))
        .collect();
    assert!(
        duplicated.is_empty(),
        "load-test.js defines these handlers twice (only the last one runs): {duplicated:?}"
    );
}
