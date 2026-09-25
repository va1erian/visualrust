//! API endpoint model and scaffolding (#71).
//!
//! An [`Endpoint`] is the API-designer view of one `vr-core` route: method,
//! `:param` path, handler name, description, declared request params and a
//! sample response. Endpoints persist as the manifest's `[[routes]]` (see
//! [`persist`]), scaffold Dyon handlers matching the #86 dispatch object
//! (see [`stub`]), and interchange through a small OpenAPI subset (see
//! [`openapi`]).
//!
//! The crate is pure model and text work: it opens no sockets and touches no
//! UI, so the server (`vr-web`) and the designer (`vr-ide`) can share it.

#![forbid(unsafe_code)]

mod error;
mod model;
mod openapi;
mod persist;
mod stub;

pub use error::ApiError;
pub use model::{DEFAULT_CONTENT_TYPE, Endpoint};
pub use openapi::{from_openapi, from_openapi_json, to_openapi, to_openapi_json};
pub use persist::{apply_endpoints, endpoints_from_manifest, load_endpoints, save_endpoints};
pub use stub::{defined_handlers, generate_stubs, handler_stub};
