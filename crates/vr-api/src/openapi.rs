//! Best-effort OpenAPI-ish JSON import and export.
//!
//! Only the subset the API editor models is mapped: `paths` -> per-method
//! operations, `operationId` as the handler, `summary` as the description,
//! `parameters`, and the first response's status plus first media-type example.
//! Anything else is ignored on import and not emitted on export, but whatever
//! this module *does* map round-trips through JSON.

use serde_json::{Map, Value, json};

use vr_core::manifest::{HttpMethod, ParamLocation, RouteParam, SampleResponse};

use crate::error::ApiError;
use crate::model::{DEFAULT_CONTENT_TYPE, Endpoint};

/// Builds an OpenAPI 3.0 document from `endpoints`.
pub fn to_openapi(endpoints: &[Endpoint], title: &str, version: &str) -> Value {
    let mut paths = Map::new();
    for endpoint in endpoints {
        let method = method_token(endpoint.method());
        let mut operation = Map::new();
        operation.insert(
            "operationId".to_owned(),
            Value::String(endpoint.handler().to_owned()),
        );
        if let Some(description) = endpoint.description() {
            operation.insert("summary".to_owned(), Value::String(description.to_owned()));
        }
        if !endpoint.params().is_empty() {
            let params = endpoint.params().iter().map(param_to_json).collect();
            operation.insert("parameters".to_owned(), Value::Array(params));
        }

        let response = endpoint.response();
        let mut media = Map::new();
        if let Some(body) = &response.body {
            media.insert("example".to_owned(), example_value(body));
        }
        let mut content = Map::new();
        content.insert(response.content_type.clone(), Value::Object(media));
        let mut response_object = Map::new();
        response_object.insert(
            "description".to_owned(),
            Value::String(format!("status {}", response.status)),
        );
        response_object.insert("content".to_owned(), Value::Object(content));
        let mut responses = Map::new();
        responses.insert(response.status.to_string(), Value::Object(response_object));
        operation.insert("responses".to_owned(), Value::Object(responses));

        let path_item = paths
            .entry(endpoint.path().to_owned())
            .or_insert_with(|| json!({}));
        if let Value::Object(path_item) = path_item {
            path_item.insert(method.to_owned(), Value::Object(operation));
        }
    }

    json!({
        "openapi": "3.0.0",
        "info": {"title": title, "version": version},
        "paths": Value::Object(paths),
    })
}

/// Parses an OpenAPI document into endpoints, skipping anything unmapped.
pub fn from_openapi(document: &Value) -> Result<Vec<Endpoint>, ApiError> {
    let paths = document
        .get("paths")
        .and_then(Value::as_object)
        .ok_or(ApiError::MissingPaths)?;

    let mut endpoints = Vec::new();
    for (path, item) in paths {
        let item = item
            .as_object()
            .ok_or_else(|| ApiError::InvalidPathItem { path: path.clone() })?;
        for (token, operation) in item {
            let Some(method) = parse_method(token) else {
                continue;
            };
            let operation = operation
                .as_object()
                .ok_or_else(|| ApiError::InvalidOperation {
                    method: token.clone(),
                    path: path.clone(),
                })?;
            endpoints.push(operation_to_endpoint(path, method, operation));
        }
    }
    Ok(endpoints)
}

/// Parses OpenAPI JSON text.
pub fn from_openapi_json(text: &str) -> Result<Vec<Endpoint>, ApiError> {
    let document: Value = serde_json::from_str(text)?;
    from_openapi(&document)
}

/// Serializes endpoints to pretty OpenAPI JSON.
pub fn to_openapi_json(
    endpoints: &[Endpoint],
    title: &str,
    version: &str,
) -> Result<String, ApiError> {
    Ok(serde_json::to_string_pretty(&to_openapi(
        endpoints, title, version,
    ))?)
}

fn operation_to_endpoint(
    path: &str,
    method: HttpMethod,
    operation: &Map<String, Value>,
) -> Endpoint {
    let handler = operation
        .get("operationId")
        .and_then(Value::as_str)
        .filter(|name| !name.trim().is_empty())
        .map(sanitize_identifier)
        .unwrap_or_else(|| derived_handler(path, method));

    let mut endpoint = Endpoint::new(handler.clone(), method, path.to_owned(), handler);
    if let Some(summary) = operation.get("summary").and_then(Value::as_str) {
        endpoint.set_description(summary);
    }
    if let Some(params) = operation.get("parameters").and_then(Value::as_array) {
        for param in params {
            if let Some(param) = param.as_object().and_then(parse_param) {
                endpoint.add_param(param);
            }
        }
    }
    if let Some(response) = parse_response(operation) {
        endpoint.set_response(response);
    }
    endpoint
}

fn param_to_json(param: &RouteParam) -> Value {
    let mut object = Map::new();
    object.insert("name".to_owned(), Value::String(param.name.clone()));
    object.insert("in".to_owned(), Value::String(param.location.to_string()));
    object.insert("required".to_owned(), Value::Bool(param.required));
    if let Some(description) = &param.description {
        object.insert("description".to_owned(), Value::String(description.clone()));
    }
    Value::Object(object)
}

fn parse_param(param: &Map<String, Value>) -> Option<RouteParam> {
    let name = param.get("name").and_then(Value::as_str)?.to_owned();
    let location = match param.get("in").and_then(Value::as_str) {
        Some("query") => ParamLocation::Query,
        Some("body") => ParamLocation::Body,
        _ => ParamLocation::Path,
    };
    let required = param
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let description = param
        .get("description")
        .and_then(Value::as_str)
        .map(str::to_owned);
    Some(RouteParam {
        name,
        location,
        required,
        description,
    })
}

fn parse_response(operation: &Map<String, Value>) -> Option<SampleResponse> {
    let responses = operation.get("responses")?.as_object()?;
    let (code, response) = responses.iter().next()?;
    let status = code.parse::<u16>().ok()?;
    let response = response.as_object()?;

    let (content_type, body) = response
        .get("content")
        .and_then(Value::as_object)
        .and_then(|content| content.iter().next())
        .map(|(content_type, media)| {
            let body = media.get("example").map(example_body);
            (content_type.clone(), body)
        })
        .unwrap_or_else(|| (DEFAULT_CONTENT_TYPE.to_owned(), None));

    Some(SampleResponse {
        status,
        body,
        content_type,
    })
}

/// Keeps objects and arrays as JSON so the example is readable; everything else
/// stays a string so `example` can round-trip verbatim.
fn example_value(body: &str) -> Value {
    match serde_json::from_str::<Value>(body) {
        Ok(value @ Value::Object(_)) | Ok(value @ Value::Array(_)) => value,
        _ => Value::String(body.to_owned()),
    }
}

fn example_body(example: &Value) -> String {
    match example {
        Value::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn method_token(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Put => "put",
        HttpMethod::Patch => "patch",
        HttpMethod::Delete => "delete",
        HttpMethod::Head => "head",
        HttpMethod::Options => "options",
    }
}

fn parse_method(token: &str) -> Option<HttpMethod> {
    match token {
        "get" => Some(HttpMethod::Get),
        "post" => Some(HttpMethod::Post),
        "put" => Some(HttpMethod::Put),
        "patch" => Some(HttpMethod::Patch),
        "delete" => Some(HttpMethod::Delete),
        "head" => Some(HttpMethod::Head),
        "options" => Some(HttpMethod::Options),
        _ => None,
    }
}

fn derived_handler(path: &str, method: HttpMethod) -> String {
    let mut parts: Vec<String> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.trim_start_matches(':').to_owned())
        .collect();
    parts.push(method.to_string().to_ascii_lowercase());
    sanitize_identifier(&parts.join("_"))
}

/// Reduces an operation id to a manifest name and Dyon identifier. Manifests
/// accept `-` but Dyon does not, so everything outside `[A-Za-z0-9_]` becomes
/// `_`; a leading digit is prefixed to keep it a valid identifier.
fn sanitize_identifier(raw: &str) -> String {
    let mut out: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() {
        out.push('_');
    }
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{from_openapi, from_openapi_json, to_openapi};
    use crate::model::Endpoint;
    use vr_core::manifest::{HttpMethod, ParamLocation, RouteParam, SampleResponse};

    fn endpoints() -> Vec<Endpoint> {
        let mut show = Endpoint::new("show_todo", HttpMethod::Get, "/todos/:id", "show_todo");
        show.set_description("Fetch one todo");
        show.add_param(RouteParam {
            name: "id".to_owned(),
            location: ParamLocation::Path,
            required: true,
            description: None,
        });
        show.set_response(SampleResponse {
            status: 200,
            body: Some("{\"id\":1}".to_owned()),
            content_type: "application/json".to_owned(),
        });

        let mut create = Endpoint::new("create_todo", HttpMethod::Post, "/todos", "create_todo");
        create.set_response(SampleResponse {
            status: 201,
            body: Some("created".to_owned()),
            content_type: "text/plain".to_owned(),
        });

        // Paths are emitted in map (sorted) order, so keep the fixture sorted
        // by path to make the round-trip comparison exact.
        vec![create, show]
    }

    #[test]
    fn exports_the_documented_subset() {
        let document = to_openapi(&endpoints(), "Todo", "1.0");
        assert_eq!(document["openapi"], "3.0.0");
        assert_eq!(
            document["paths"]["/todos/:id"]["get"]["operationId"],
            "show_todo"
        );
        assert_eq!(
            document["paths"]["/todos/:id"]["get"]["responses"]["200"]["content"]["application/json"]
                ["example"]["id"],
            1
        );
    }

    #[test]
    fn round_trips_through_json() {
        let document = to_openapi(&endpoints(), "Todo", "1.0");
        let json = serde_json::to_string(&document).expect("serialize");
        let parsed = from_openapi_json(&json).expect("parse");
        assert_eq!(parsed, endpoints());
    }

    #[test]
    fn import_derives_a_handler_when_operation_id_is_missing() {
        let json = r#"{"paths":{"/todos/:id":{"get":{"responses":{"200":{}}}}}}"#;
        let endpoints = from_openapi_json(json).expect("parse");
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].handler(), "todos_id_get");
        assert_eq!(endpoints[0].name(), "todos_id_get");
    }

    #[test]
    fn rejects_a_document_without_paths() {
        assert!(from_openapi(&serde_json::json!({"openapi": "3.0.0"})).is_err());
    }
}
