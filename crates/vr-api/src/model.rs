//! The endpoint model, layered over [`vr_core::manifest::Route`].
//!
//! [`Endpoint`] is a thin newtype rather than a parallel struct so the manifest
//! stays the single source of truth: the API editor edits endpoints, and they
//! serialize straight back into `[[routes]]` without a lossy mapping.

use std::collections::HashSet;

use vr_core::manifest::{HttpMethod, Route, RouteParam, SampleResponse};

use crate::error::ApiError;

/// The content type a scaffolded handler returns when the endpoint names none.
pub const DEFAULT_CONTENT_TYPE: &str = "text/plain; charset=utf-8";

/// One HTTP endpoint as edited in the API designer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    route: Route,
}

impl Endpoint {
    /// A bare endpoint with no description, params or sample response.
    pub fn new(
        name: impl Into<String>,
        method: HttpMethod,
        path: impl Into<String>,
        handler: impl Into<String>,
    ) -> Self {
        Self {
            route: Route {
                name: name.into(),
                method,
                path: path.into(),
                handler: handler.into(),
                description: None,
                params: Vec::new(),
                response: None,
            },
        }
    }

    /// Wraps an already-parsed manifest route.
    pub fn from_route(route: Route) -> Self {
        Self { route }
    }

    /// Drops the wrapper, leaving the value that belongs in the manifest.
    pub fn into_route(self) -> Route {
        self.route
    }

    /// The underlying route.
    pub fn route(&self) -> &Route {
        &self.route
    }

    /// Mutable access for bulk edits; callers should re-run [`validate`].
    pub fn route_mut(&mut self) -> &mut Route {
        &mut self.route
    }

    /// The endpoint's unique manifest name.
    pub fn name(&self) -> &str {
        &self.route.name
    }

    /// The HTTP verb.
    pub fn method(&self) -> HttpMethod {
        self.route.method
    }

    /// The path pattern, with `:name` captures.
    pub fn path(&self) -> &str {
        &self.route.path
    }

    /// The Dyon handler function this endpoint dispatches to.
    pub fn handler(&self) -> &str {
        &self.route.handler
    }

    /// The human-readable summary, if any.
    pub fn description(&self) -> Option<&str> {
        self.route.description.as_deref()
    }

    /// Declared request parameters.
    pub fn params(&self) -> &[RouteParam] {
        &self.route.params
    }

    /// The sample response, falling back to a `200` with an empty text body.
    pub fn response(&self) -> SampleResponse {
        self.route.response.clone().unwrap_or_default()
    }

    /// Replaces the handler name.
    pub fn set_handler(&mut self, handler: impl Into<String>) -> &mut Self {
        self.route.handler = handler.into();
        self
    }

    /// Sets the summary.
    pub fn set_description(&mut self, description: impl Into<String>) -> &mut Self {
        self.route.description = Some(description.into());
        self
    }

    /// Replaces the sample response.
    pub fn set_response(&mut self, response: SampleResponse) -> &mut Self {
        self.route.response = Some(response);
        self
    }

    /// Appends a declared request parameter.
    pub fn add_param(&mut self, param: RouteParam) -> &mut Self {
        self.route.params.push(param);
        self
    }

    /// The `:name` captures in the path, in order.
    pub fn path_params(&self) -> Vec<&str> {
        self.route
            .path
            .split('/')
            .filter_map(|segment| segment.strip_prefix(':'))
            .filter(|name| !name.is_empty())
            .collect()
    }

    /// Checks the invariants the router and manifest both rely on.
    pub fn validate(&self) -> Result<(), ApiError> {
        if self.route.handler.trim().is_empty() {
            return Err(ApiError::EmptyHandler);
        }
        if !self.route.path.starts_with('/') {
            return Err(ApiError::InvalidPath {
                path: self.route.path.clone(),
            });
        }
        let mut seen = HashSet::new();
        for param in self.path_params() {
            if !seen.insert(param) {
                return Err(ApiError::DuplicateParam {
                    name: self.route.name.clone(),
                    param: param.to_owned(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Endpoint;
    use vr_core::manifest::HttpMethod;

    #[test]
    fn extracts_path_params_in_order() {
        let endpoint = Endpoint::new("show", HttpMethod::Get, "/users/:id/posts/:post", "show");
        assert_eq!(endpoint.path_params(), vec!["id", "post"]);
    }

    #[test]
    fn rejects_a_path_without_a_leading_slash() {
        let endpoint = Endpoint::new("bad", HttpMethod::Get, "users", "bad");
        assert!(endpoint.validate().is_err());
    }

    #[test]
    fn rejects_a_duplicate_path_param() {
        let endpoint = Endpoint::new("bad", HttpMethod::Get, "/:id/:id", "bad");
        assert!(endpoint.validate().is_err());
    }

    #[test]
    fn round_trips_through_a_route() {
        let mut endpoint = Endpoint::new("index", HttpMethod::Get, "/", "index");
        endpoint.set_description("home");
        let route = endpoint.clone().into_route();
        assert_eq!(Endpoint::from_route(route), endpoint);
    }
}
