//! The Dyon binding: `server_*` natives plus the off-thread runtime bridge.
//!
//! # Model
//!
//! Dyon is single-threaded: one [`dyon::Runtime`] is owned by exactly one
//! thread, [`WebRuntime`], and handler functions are only ever called from that
//! thread. The HTTP [`Server`](crate::Server) runs a pool of worker threads on
//! a *different* thread; a worker never touches Dyon. Instead it posts a
//! [`Call`](bridge) — the handler name plus the [`Request`](crate::Request) —
//! over a channel and blocks on a one-shot reply, and the runtime thread calls
//! the handler and sends the [`Response`](crate::Response) back. Because the
//! runtime thread services calls one at a time and a handler never re-enters
//! the bridge, Dyon is neither shared nor re-entered.
//!
//! [`server_start`](#natives) blocks the Dyon program in that service loop
//! until shutdown; that mirrors the xui `ui_run` model where a native call
//! owns the dispatch loop.
//!
//! # Natives
//!
//! Registered by [`register`], all taking the server handle value returned by
//! `server_new`:
//!
//! * `server_new() -> Server` — a fresh route builder.
//! * `server_route(server, method, pattern, handler) -> ()` — `pattern` uses
//!   `:name` captures; `handler` is a Dyon function name, never a closure.
//! * `server_start(server) -> ()` — binds `127.0.0.1:0`, starts the HTTP
//!   workers and blocks in the service loop until the server stops.
//! * `server_stop(server) -> ()` — requests shutdown; the loop returns once the
//!   in-flight requests have drained.
//!
//! # Response mapping
//!
//! A handler takes one Dyon object and returns one Dyon object:
//!
//! ```dyon
//! fn hello(req: {}) -> {} {
//!     return {status: 200, body: "hi", content_type: "text/plain"}
//! }
//! ```
//!
//! The request object has `method`, `path`, `body` (lossy UTF-8), and the
//! `query` / `params` / `headers` sub-objects, all string-valued. The response
//! object needs `status` (a whole number in `100..=599`) and may set `body` and
//! `content_type` (default `text/plain; charset=utf-8`). Anything else is a
//! [`DyonResponseError`] and becomes a `500` rather than a panic.

mod bridge;
mod convert;

use std::sync::Arc;

use ::dyon::embed::to_rust_object;
use ::dyon::{Dfn, Module, Runtime, RustObject, Type, Variable};

use crate::method::Method;
use crate::server::Shutdown;

pub use bridge::{BridgeError, WebRuntime};
pub use convert::DyonResponseError;

/// The server handle a Dyon program builds routes on.
struct ServerPlan {
    routes: Vec<RouteSpec>,
    shutdown: Shutdown,
}

/// One route: a method, a `:param` pattern and the handler function name.
#[derive(Clone)]
struct RouteSpec {
    method: Method,
    pattern: String,
    handler: String,
}

/// Registers the `server_*` natives into `module`.
///
/// Must run before source is loaded so Dyon's lifetime checker sees the
/// signatures, exactly like `vr-dyon`'s built-ins.
pub fn register(module: &mut Module) {
    let server_ty = ad_hoc("Server");
    module.add_str("server_new", server_new, Dfn::nl(vec![], server_ty.clone()));
    module.add_str(
        "server_route",
        server_route,
        Dfn::nl(
            vec![server_ty.clone(), Type::Str, Type::Str, Type::Str],
            Type::Void,
        ),
    );
    module.add_str(
        "server_start",
        server_start,
        Dfn::nl(vec![server_ty.clone()], Type::Void),
    );
    module.add_str(
        "server_stop",
        server_stop,
        Dfn::nl(vec![server_ty], Type::Void),
    );
}

/// An ad-hoc type named `name`, so a server handle is accepted only where a
/// server handle is expected.
fn ad_hoc(name: &str) -> Type {
    Type::AdHoc(Arc::new(name.to_owned()), Box::new(Type::Any))
}

fn server_new(_runtime: &mut Runtime) -> Result<Variable, String> {
    // Sharing the bridge's shutdown flag lets the host (`WebRuntime`) stop a
    // running program from outside, without a route that calls `server_stop`.
    let shutdown = bridge::current()
        .map(|state| state.shutdown.clone())
        .unwrap_or_default();
    Ok(Variable::RustObject(to_rust_object(ServerPlan {
        routes: Vec::new(),
        shutdown,
    })))
}

fn server_route(runtime: &mut Runtime) -> Result<(), String> {
    let handler: String = runtime.pop()?;
    let pattern: String = runtime.pop()?;
    let method_token: String = runtime.pop()?;
    let server: RustObject = runtime.pop()?;
    let method = Method::parse(&method_token)
        .ok_or_else(|| format!("unsupported HTTP method `{method_token}`"))?;
    with_plan_mut(&server, |plan| {
        plan.routes.push(RouteSpec {
            method,
            pattern,
            handler,
        })
    })
}

fn server_start(runtime: &mut Runtime) -> Result<(), String> {
    let server: RustObject = runtime.pop()?;
    let (routes, shutdown) =
        with_plan(&server, |plan| (plan.routes.clone(), plan.shutdown.clone()))?;
    bridge::serve(runtime, routes, shutdown).map_err(|error| error.to_string())
}

fn server_stop(runtime: &mut Runtime) -> Result<(), String> {
    let server: RustObject = runtime.pop()?;
    with_plan(&server, |plan| plan.shutdown.trigger())?;
    Ok(())
}

fn with_plan<T>(server: &RustObject, read: impl FnOnce(&ServerPlan) -> T) -> Result<T, String> {
    let guard = server
        .lock()
        .map_err(|_| "the server handle was poisoned".to_owned())?;
    let plan = guard
        .downcast_ref::<ServerPlan>()
        .ok_or_else(|| "expected a server handle".to_owned())?;
    Ok(read(plan))
}

fn with_plan_mut(server: &RustObject, write: impl FnOnce(&mut ServerPlan)) -> Result<(), String> {
    let mut guard = server
        .lock()
        .map_err(|_| "the server handle was poisoned".to_owned())?;
    let plan = guard
        .downcast_mut::<ServerPlan>()
        .ok_or_else(|| "expected a server handle".to_owned())?;
    write(plan);
    Ok(())
}
