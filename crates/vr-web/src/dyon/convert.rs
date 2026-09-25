//! Translation between HTTP messages and the small Dyon object shapes the
//! binding documents. Kept separate from the bridge so the mapping is testable
//! without threads or sockets.

// Dyon's own `Object` type is `Arc<HashMap<..>>` even though `Variable` holds an
// `UnsafeRef` and is therefore not `Sync`; the crate builds objects this way and
// these values never cross a thread boundary, so the `Rc` suggestion would only
// fight Dyon's public API.
#![allow(clippy::arc_with_non_send_sync)]

use std::collections::HashMap;
use std::sync::Arc;

use ::dyon::Variable;
use thiserror::Error;

use crate::request::Request;
use crate::response::{Response, StatusCode};

/// The documented default when a handler omits `content_type`.
const DEFAULT_CONTENT_TYPE: &str = "text/plain; charset=utf-8";

/// Why a Dyon handler's return value was not a usable response.
///
/// These are the only ways mapping can fail; the bridge turns them into a `500`
/// and records the most recent one on the [`WebRuntime`](super::WebRuntime).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DyonResponseError {
    /// The handler call itself raised a Dyon runtime error.
    #[error("dyon handler call failed: {0}")]
    Call(String),
    /// The handler returned a number, string, or anything but an object.
    #[error("handler returned {found}, but a response object is required")]
    NotAnObject { found: String },
    /// The response object had no `status` key.
    #[error("response object has no `status`")]
    MissingStatus,
    /// `status` was not a whole number in `100..=599`.
    #[error("response `status` must be a whole number in 100..=599")]
    InvalidStatus,
    /// `body` was present but not a string.
    #[error("response `body` must be a string")]
    InvalidBody,
    /// `content_type` was present but not a string.
    #[error("response `content_type` must be a string")]
    InvalidContentType,
}

/// Builds the Dyon object handed to a handler function.
///
/// The body is decoded lossily because Dyon strings are UTF-8 and the HTTP
/// layer carries bytes; a binary body would need a separate array encoding that
/// no handler in this slice needs yet.
pub(crate) fn request_object(request: &Request) -> Variable {
    let mut object = HashMap::new();
    object.insert(
        key("method"),
        Variable::Str(Arc::new(request.method.as_str().to_owned())),
    );
    object.insert(key("path"), Variable::Str(Arc::new(request.path.clone())));
    object.insert(
        key("body"),
        Variable::Str(Arc::new(
            String::from_utf8_lossy(&request.body).into_owned(),
        )),
    );
    object.insert(key("query"), string_map(&request.query));
    object.insert(key("params"), string_map(&request.params));
    let mut headers = HashMap::new();
    for (name, value) in request.headers.iter() {
        headers.insert(key(name), Variable::Str(Arc::new(value.to_owned())));
    }
    object.insert(key("headers"), Variable::Object(Arc::new(headers)));
    Variable::Object(Arc::new(object))
}

/// Converts a handler's return value into a [`Response`].
pub(crate) fn response_from_variable(value: &Variable) -> Result<Response, DyonResponseError> {
    let Variable::Object(object) = value else {
        return Err(DyonResponseError::NotAnObject {
            found: value.typeof_var().to_string(),
        });
    };

    let status = match object.get(&key("status")) {
        Some(&Variable::F64(code, _)) if is_valid_status(code) => code as u16,
        Some(Variable::F64(_, _)) => return Err(DyonResponseError::InvalidStatus),
        Some(_) => return Err(DyonResponseError::InvalidStatus),
        None => return Err(DyonResponseError::MissingStatus),
    };

    let body = match object.get(&key("body")) {
        Some(Variable::Str(text)) => text.as_bytes().to_vec(),
        Some(_) => return Err(DyonResponseError::InvalidBody),
        None => Vec::new(),
    };

    let content_type = match object.get(&key("content_type")) {
        Some(Variable::Str(text)) => text.to_string(),
        Some(_) => return Err(DyonResponseError::InvalidContentType),
        None => DEFAULT_CONTENT_TYPE.to_owned(),
    };

    let mut response = Response::new(StatusCode::from_u16(status));
    response.headers.insert("content-type", content_type);
    response.body = body;
    Ok(response)
}

fn is_valid_status(code: f64) -> bool {
    code.is_finite() && code.fract() == 0.0 && (100.0..=599.0).contains(&code)
}

fn string_map(source: &std::collections::BTreeMap<String, String>) -> Variable {
    let mut object = HashMap::new();
    for (name, value) in source {
        object.insert(key(name), Variable::Str(Arc::new(value.clone())));
    }
    Variable::Object(Arc::new(object))
}

fn key(name: &str) -> Arc<String> {
    Arc::new(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{DyonResponseError, request_object, response_from_variable};
    use crate::{Headers, Method, Request, StatusCode};
    use std::collections::HashMap;
    use std::sync::Arc;

    fn object(entries: Vec<(&str, ::dyon::Variable)>) -> ::dyon::Variable {
        let map: HashMap<Arc<String>, ::dyon::Variable> = entries
            .into_iter()
            .map(|(name, value)| (Arc::new(name.to_owned()), value))
            .collect();
        ::dyon::Variable::Object(Arc::new(map))
    }

    #[test]
    fn maps_a_status_body_and_content_type() {
        let value = object(vec![
            ("status", ::dyon::Variable::F64(201.0, None)),
            ("body", ::dyon::Variable::Str(Arc::new("made".to_owned()))),
            (
                "content_type",
                ::dyon::Variable::Str(Arc::new("application/json".to_owned())),
            ),
        ]);
        let response = response_from_variable(&value).expect("valid response");
        assert_eq!(response.status, StatusCode::from_u16(201));
        assert_eq!(response.body, b"made");
        assert_eq!(
            response.headers.get("content-type"),
            Some("application/json")
        );
    }

    #[test]
    fn defaults_content_type_and_body() {
        let value = object(vec![("status", ::dyon::Variable::F64(204.0, None))]);
        let response = response_from_variable(&value).expect("valid response");
        assert_eq!(response.status, StatusCode::NO_CONTENT);
        assert!(response.body.is_empty());
        assert_eq!(
            response.headers.get("content-type"),
            Some("text/plain; charset=utf-8")
        );
    }

    #[test]
    fn rejects_malformed_returns() {
        let not_object = ::dyon::Variable::F64(1.0, None);
        assert!(matches!(
            response_from_variable(&not_object),
            Err(DyonResponseError::NotAnObject { .. })
        ));

        assert_eq!(
            response_from_variable(&object(vec![])).unwrap_err(),
            DyonResponseError::MissingStatus
        );

        let bad_status = object(vec![("status", ::dyon::Variable::F64(42.0, None))]);
        assert_eq!(
            response_from_variable(&bad_status).unwrap_err(),
            DyonResponseError::InvalidStatus
        );

        let bad_body = object(vec![
            ("status", ::dyon::Variable::F64(200.0, None)),
            ("body", ::dyon::Variable::F64(1.0, None)),
        ]);
        assert_eq!(
            response_from_variable(&bad_body).unwrap_err(),
            DyonResponseError::InvalidBody
        );
    }

    #[test]
    fn request_object_carries_method_path_query_and_params() {
        let mut request = Request::new(Method::Get, "/users/7");
        request.query.insert("q".to_owned(), "hi".to_owned());
        request.params.insert("id".to_owned(), "7".to_owned());
        let mut headers = Headers::new();
        headers.insert("host", "example");
        request.headers = headers;
        request.body = b"payload".to_vec();

        let ::dyon::Variable::Object(object) = request_object(&request) else {
            panic!("request object must be an object");
        };
        assert_eq!(object[&Arc::new("method".to_owned())].type_string(), "GET");
        assert_eq!(
            object[&Arc::new("path".to_owned())].type_string(),
            "/users/7"
        );
        assert_eq!(
            object[&Arc::new("body".to_owned())].type_string(),
            "payload"
        );
    }

    trait TextOf {
        fn type_string(&self) -> String;
    }

    impl TextOf for ::dyon::Variable {
        fn type_string(&self) -> String {
            match self {
                ::dyon::Variable::Str(text) => text.to_string(),
                other => format!("<{}>", other.typeof_var()),
            }
        }
    }
}
