//! Method + path-pattern routing to a handler name.

use std::collections::BTreeMap;
use std::sync::Arc;

use thiserror::Error;

use crate::handler::{Handler, HandlerRegistry};
use crate::method::Method;

/// Errors from defining routes or handlers.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RouterError {
    /// The pattern was the empty string.
    #[error("route pattern must not be empty")]
    EmptyPattern,
    /// A `:` parameter had no name, as in `/users/:`.
    #[error("route parameter name is empty in `{0}`")]
    EmptyParam(String),
    /// The same parameter name appeared twice in one pattern.
    #[error("route parameter `{0}` appears more than once")]
    DuplicateParam(String),
    /// A handler with that name was already registered.
    #[error("a handler named `{0}` is already registered")]
    DuplicateHandler(String),
}

/// One path segment of a compiled pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Literal(String),
    Param(String),
}

#[derive(Debug, Clone)]
struct Route {
    method: Method,
    segments: Vec<Segment>,
    handler: String,
}

/// The result of matching a request against the route table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteMatch {
    /// The name of the handler the route points at.
    pub handler: String,
    /// Captured `:params`, keyed by name.
    pub params: BTreeMap<String, String>,
}

/// What [`Router::resolve`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// A route matched both method and path.
    Matched(RouteMatch),
    /// No route matched the path.
    NotFound,
    /// A route matched the path but not the method; the payload lists the
    /// methods that would have worked, for the `allow` header.
    MethodNotAllowed { allowed: Vec<Method> },
}

/// A route table plus the named handlers it points at.
///
/// `Default` is implemented via [`Router::new`] so a router can be built with
/// struct-update syntax where convenient.
pub struct Router {
    routes: Vec<Route>,
    handlers: HandlerRegistry,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    /// An empty router.
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            handlers: HandlerRegistry::new(),
        }
    }

    /// Adds a route. `pattern` uses `:name` for a captured segment, e.g.
    /// `/users/:id`. Returns `&mut Self` for chaining.
    pub fn route(
        &mut self,
        method: Method,
        pattern: impl Into<String>,
        handler: impl Into<String>,
    ) -> Result<&mut Self, RouterError> {
        let pattern = pattern.into();
        let segments = compile_pattern(&pattern)?;
        self.routes.push(Route {
            method,
            segments,
            handler: handler.into(),
        });
        Ok(self)
    }

    /// Registers a handler under a name. The name does not have to be routed
    /// yet, and a route may name a handler that is registered later.
    pub fn register_handler(
        &mut self,
        name: impl Into<String>,
        handler: impl Handler + 'static,
    ) -> Result<&mut Self, RouterError> {
        let name = name.into();
        if !self.handlers.register(name.clone(), handler) {
            return Err(RouterError::DuplicateHandler(name));
        }
        Ok(self)
    }

    /// Resolves a handler name to its implementation.
    pub fn handler(&self, name: &str) -> Option<Arc<dyn Handler>> {
        self.handlers.get(name)
    }

    /// Matches a method and decoded path.
    ///
    /// Paths are split on `/` with empty segments skipped, so `/a/` and `/a`
    /// match the same route. A method mismatch is only reported once the whole
    /// table has been scanned, which is what distinguishes `405` from `404`.
    pub fn resolve(&self, method: Method, path: &str) -> Resolution {
        let request_segments = split_path(path);
        let mut allowed = Vec::new();
        for route in &self.routes {
            if let Some(params) = match_route(route, &request_segments) {
                if route.method == method {
                    return Resolution::Matched(RouteMatch {
                        handler: route.handler.clone(),
                        params,
                    });
                }
                if !allowed.contains(&route.method) {
                    allowed.push(route.method);
                }
            }
        }
        if allowed.is_empty() {
            Resolution::NotFound
        } else {
            Resolution::MethodNotAllowed { allowed }
        }
    }

    /// The number of registered routes.
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Whether no routes are registered.
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}

/// Splits a path, dropping empty segments so `//a` and `/a/` are equivalent.
fn split_path(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn compile_pattern(pattern: &str) -> Result<Vec<Segment>, RouterError> {
    if pattern.is_empty() {
        return Err(RouterError::EmptyPattern);
    }
    let mut segments = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for raw in pattern.split('/') {
        if raw.is_empty() {
            continue;
        }
        match raw.strip_prefix(':') {
            Some(name) => {
                if name.is_empty() {
                    return Err(RouterError::EmptyParam(pattern.to_owned()));
                }
                if seen.contains(&name) {
                    return Err(RouterError::DuplicateParam(name.to_owned()));
                }
                seen.push(name);
                segments.push(Segment::Param(name.to_owned()));
            }
            None => segments.push(Segment::Literal(raw.to_owned())),
        }
    }
    Ok(segments)
}

fn match_route(route: &Route, request: &[&str]) -> Option<BTreeMap<String, String>> {
    if route.segments.len() != request.len() {
        return None;
    }
    let mut params = BTreeMap::new();
    for (segment, part) in route.segments.iter().zip(request) {
        match segment {
            Segment::Literal(literal) if literal == part => {}
            Segment::Literal(_) => return None,
            Segment::Param(name) if !part.is_empty() => {
                params.insert(name.clone(), (*part).to_owned());
            }
            Segment::Param(_) => return None,
        }
    }
    Some(params)
}

#[cfg(test)]
mod tests {
    use super::{Resolution, RouteMatch, Router, RouterError};
    use crate::Method;
    use std::collections::BTreeMap;

    fn router() -> Router {
        let mut router = Router::new();
        router
            .route(Method::Get, "/users/:id", "show_user")
            .expect("route");
        router
            .route(Method::Post, "/users", "create_user")
            .expect("route");
        router.route(Method::Get, "/", "index").expect("route");
        router
    }

    fn matched(handler: &str, params: &[(&str, &str)]) -> Resolution {
        let params = params
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect::<BTreeMap<_, _>>();
        Resolution::Matched(RouteMatch {
            handler: handler.to_owned(),
            params,
        })
    }

    #[test]
    fn captures_params() {
        assert_eq!(
            router().resolve(Method::Get, "/users/42"),
            matched("show_user", &[("id", "42")])
        );
    }

    #[test]
    fn matches_root() {
        assert_eq!(router().resolve(Method::Get, "/"), matched("index", &[]));
    }

    #[test]
    fn ignores_trailing_slash() {
        assert_eq!(
            router().resolve(Method::Get, "/users/42/"),
            matched("show_user", &[("id", "42")])
        );
    }

    #[test]
    fn reports_not_found() {
        assert_eq!(
            router().resolve(Method::Get, "/missing"),
            Resolution::NotFound
        );
    }

    #[test]
    fn reports_method_not_allowed_with_options() {
        assert_eq!(
            router().resolve(Method::Delete, "/users"),
            Resolution::MethodNotAllowed {
                allowed: vec![Method::Post]
            }
        );
    }

    #[test]
    fn rejects_empty_and_duplicate_params() {
        let mut router = Router::new();
        assert!(matches!(
            router.route(Method::Get, "/a/:", "h"),
            Err(RouterError::EmptyParam(_))
        ));
        assert!(matches!(
            router.route(Method::Get, "/:id/:id", "h"),
            Err(RouterError::DuplicateParam(_))
        ));
        assert!(matches!(
            router.route(Method::Get, "", "h"),
            Err(RouterError::EmptyPattern)
        ));
    }
}
