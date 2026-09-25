//! A small, blocking, message-oriented HTTP/1.1 server core.
//!
//! The crate turns each incoming request into a [`Request`] message and hands it
//! to a handler registered under a name. Routing and handler storage are kept
//! deliberately separate so the Dyon binding can arrive later without touching
//! the HTTP machinery:
//!
//! * [`Router`] matches `(method, path-with-:params)` to a **handler name** and
//!   reports `404` (no path) or `405` (path exists, method does not).
//! * [`HandlerRegistry`] stores [`Handler`] implementations under those names.
//! * [`Server`] runs a bounded pool of worker threads; each worker parses a
//!   request, asks the router for a handler name, looks the handler up and
//!   writes its [`Response`].
//!
//! # The Dyon seam (#86)
//!
//! The seam is the **handler name**. `Router::route` records a `String`, never a
//! closure, and `Handler` is a trait rather than a `Fn` type, so the Dyon
//! binding in [`dyon`] keeps the router as-is and maps each name to the owning
//! runtime thread. A worker sends the [`Request`] to that thread and waits for
//! the [`Response`], which matches the win32ui `Msg` model and keeps Dyon
//! single-threaded; see [`dyon`] for the `server_*` natives and the bridge.
//!
//! The HTTP parser is hand-rolled on purpose: the required subset is small
//! (request line, headers, `Content-Length` body) and avoids pulling a
//! framework into the self-contained runtime build.

#![forbid(unsafe_code)]

pub mod dyon;
mod handler;
mod headers;
mod method;
mod parse;
mod request;
mod response;
mod router;
mod server;

pub use dyon::{BridgeError, DyonResponseError, WebRuntime};

pub use handler::{Handler, HandlerRegistry};
pub use headers::Headers;
pub use method::Method;
pub use parse::{ParseError, ParseLimits, parse_request};
pub use request::Request;
pub use response::{Response, StatusCode};
pub use router::{Resolution, RouteMatch, Router, RouterError};
pub use server::{Server, ServerError, Shutdown};
