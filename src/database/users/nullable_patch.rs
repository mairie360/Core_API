use mairie360_api_lib::database::db_interface::QueryParam;

// A partial update of a nullable column takes three parameters, bound in this order:
// `provided` (the field was in the body), `is_null` (it was `null`, reset to the default) and the
// value itself (a placeholder when absent or `null`). The SQL keeps the column when `provided` is
// false, writes NULL when `is_null` is true, and the value otherwise.

/// Pushes the three parameters of a nullable text column.
#[allow(clippy::option_option)]
pub fn push_text(params: &mut Vec<QueryParam>, field: Option<Option<&str>>) {
    params.push(QueryParam::Bool(field.is_some()));
    params.push(QueryParam::Bool(matches!(field, Some(None))));
    params.push(QueryParam::Text(
        field.flatten().unwrap_or_default().to_string(),
    ));
}

/// Pushes the three parameters of a nullable integer column.
#[allow(clippy::option_option)]
pub fn push_i32(params: &mut Vec<QueryParam>, field: Option<Option<i32>>) {
    params.push(QueryParam::Bool(field.is_some()));
    params.push(QueryParam::Bool(matches!(field, Some(None))));
    params.push(QueryParam::I32(field.flatten().unwrap_or_default()));
}

/// Pushes the three parameters of a nullable boolean column.
#[allow(clippy::option_option)]
pub fn push_bool(params: &mut Vec<QueryParam>, field: Option<Option<bool>>) {
    params.push(QueryParam::Bool(field.is_some()));
    params.push(QueryParam::Bool(matches!(field, Some(None))));
    params.push(QueryParam::Bool(field.flatten().unwrap_or_default()));
}
