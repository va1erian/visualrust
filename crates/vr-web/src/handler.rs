//! Named handler storage: the seam the Dyon binding plugs into.

use std::collections::HashMap;
use std::sync::Arc;

use crate::request::Request;
use crate::response::Response;

/// Something that turns a [`Request`] into a [`Response`].
///
/// This is a trait, not a closure type, because the Dyon binding (#86) needs to
/// register a name whose call crosses a thread boundary. A blanket impl below
/// lets Rust tests register plain closures.
pub trait Handler: Send + Sync {
    /// Handles one request. Called on a worker thread.
    fn handle(&self, request: &Request) -> Response;
}

impl<F> Handler for F
where
    F: Fn(&Request) -> Response + Send + Sync,
{
    fn handle(&self, request: &Request) -> Response {
        self(request)
    }
}

/// Handlers keyed by name.
///
/// Route patterns store a name; the registry resolves it. Swapping this for a
/// map from name to a Dyon function is exactly the #86 change, so no route has
/// to be rewritten.
#[derive(Default)]
pub struct HandlerRegistry {
    handlers: HashMap<String, Arc<dyn Handler>>,
}

impl HandlerRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handler. Returns `false` (leaving the old handler in place)
    /// when the name is already taken, so a caller can report a duplicate.
    pub fn register(&mut self, name: impl Into<String>, handler: impl Handler + 'static) -> bool {
        let name = name.into();
        if self.handlers.contains_key(&name) {
            return false;
        }
        self.handlers.insert(name, Arc::new(handler));
        true
    }

    /// Looks up a handler by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Handler>> {
        self.handlers.get(name).cloned()
    }

    /// Whether a name is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.handlers.contains_key(name)
    }

    /// The number of registered handlers.
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Whether no handlers are registered.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    /// The registered names, in unspecified order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.handlers.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::HandlerRegistry;
    use crate::{Request, Response, StatusCode};

    #[test]
    fn keeps_the_first_handler_on_duplicate_name() {
        let mut registry = HandlerRegistry::new();
        assert!(registry.register("a", |_: &Request| Response::text(StatusCode::OK, "first")));
        assert!(!registry.register("a", |_: &Request| Response::text(StatusCode::OK, "second")));
        assert_eq!(registry.len(), 1);
    }
}
