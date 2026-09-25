//! The off-thread runtime bridge.
//!
//! One thread owns the Dyon runtime; HTTP workers only ever post a [`Call`] and
//! wait for the reply. See the module docs in [`super`] for the model.

use std::cell::RefCell;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::mpsc::{Receiver, Sender, SyncSender, channel, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use thiserror::Error;

use crate::dyon::RouteSpec;
use crate::dyon::convert::{DyonResponseError, request_object, response_from_variable};
use crate::handler::Handler;
use crate::request::Request;
use crate::response::{Response, StatusCode};
use crate::router::{Router, RouterError};
use crate::server::{Server, ServerError, Shutdown};

use vr_dyon::{DyonError, DyonRuntime};

/// Worker count for the HTTP pool. Small: the runtime thread, not the pool, is
/// the serialisation point, so extra workers only cover socket IO.
const POOL_SIZE: usize = 4;
/// The binding always serves loopback on an ephemeral port; a deployment host
/// would expose the address from `server_start` instead.
const BIND_ADDR: &str = "127.0.0.1:0";

/// Anything that stops the web runtime from starting or serving.
#[derive(Debug, Error)]
pub enum BridgeError {
    /// The Dyon program failed to compile or raised while running `main`.
    #[error("dyon error: {0}")]
    Dyon(#[from] DyonError),
    /// A route or handler name was invalid or duplicated.
    #[error("route error: {0}")]
    Route(#[from] RouterError),
    /// The HTTP server could not bind or run.
    #[error("server error: {0}")]
    Server(#[from] ServerError),
    /// A thread could not be spawned.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// `server_start` was reached before any route was registered.
    #[error("the Dyon program registered no routes before starting the server")]
    NoRoutes,
    /// `main` returned without calling `server_start`.
    #[error("the Dyon program did not start a server")]
    ServerNotStarted,
    /// `main` failed before a server existed.
    #[error("the Dyon program failed before starting: {0}")]
    Program(String),
    /// A runtime or HTTP thread panicked.
    #[error("a web runtime thread panicked")]
    ThreadPanicked,
    /// A lock shared with a native call was poisoned.
    #[error("a web runtime lock was poisoned")]
    Poisoned,
}

/// State shared between the host thread, the Dyon runtime thread and the
/// `server_*` natives.
pub(crate) struct BridgeShared {
    pub(crate) shutdown: Shutdown,
    addr_tx: Mutex<Option<SyncSender<Result<SocketAddr, BridgeError>>>>,
    last_error: Mutex<Option<DyonResponseError>>,
}

impl BridgeShared {
    fn new() -> Self {
        Self {
            shutdown: Shutdown::new(),
            addr_tx: Mutex::new(None),
            last_error: Mutex::new(None),
        }
    }

    /// Hands the bound address (or a startup failure) to `WebRuntime::start`.
    /// The one-shot sender is taken, so a later call is a no-op.
    fn publish_addr(&self, result: Result<SocketAddr, BridgeError>) {
        let Ok(mut guard) = self.addr_tx.lock() else {
            return;
        };
        if let Some(sender) = guard.take() {
            let _ = sender.send(result);
        }
    }

    fn record(&self, error: DyonResponseError) {
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = Some(error);
        }
    }

    fn last_error(&self) -> Option<DyonResponseError> {
        self.last_error.lock().ok().and_then(|guard| guard.clone())
    }
}

thread_local! {
    static BRIDGE: RefCell<Option<Arc<BridgeShared>>> = const { RefCell::new(None) };
}

/// Installs the shared state for the current thread; called on the runtime
/// thread before the Dyon program runs.
fn install(shared: Arc<BridgeShared>) {
    BRIDGE.with(|cell| *cell.borrow_mut() = Some(shared));
}

/// The shared state of the current thread, when a `WebRuntime` owns it.
pub(crate) fn current() -> Option<Arc<BridgeShared>> {
    BRIDGE.with(|cell| cell.borrow().clone())
}

/// One request to run on the Dyon thread, with the channel to reply on.
struct Call {
    function: String,
    request: Request,
    reply: Sender<Response>,
}

/// A [`Handler`] that forwards to the Dyon runtime thread instead of running
/// Dyon inline. It is `Send + Sync`; the `Sender` is the only shared state.
struct DyonCallHandler {
    function: String,
    calls: Sender<Call>,
}

impl Handler for DyonCallHandler {
    fn handle(&self, request: &Request) -> Response {
        let (reply, reply_rx) = channel();
        let call = Call {
            function: self.function.clone(),
            request: request.clone(),
            reply,
        };
        if self.calls.send(call).is_err() {
            return Response::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                "the Dyon runtime is not available",
            );
        }
        match reply_rx.recv() {
            Ok(response) => response,
            Err(_) => Response::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                "the Dyon runtime dropped the request",
            ),
        }
    }
}

/// A Dyon program running as a web server on its own thread.
///
/// [`start`](Self::start) returns once `server_start` has bound the listener, so
/// the caller can issue real HTTP requests. Dyon keeps running on the runtime
/// thread until [`shutdown`](Self::shutdown) (or a route that calls
/// `server_stop`) drains the in-flight requests and [`join`](Self::join)
/// returns.
pub struct WebRuntime {
    shared: Arc<BridgeShared>,
    addr: SocketAddr,
    thread: Option<JoinHandle<Result<(), BridgeError>>>,
}

impl WebRuntime {
    /// Compiles and runs `source` with only the built-in natives.
    pub fn start(source_name: &str, source: &str) -> Result<Self, BridgeError> {
        Self::start_with(source_name, source, |_| {})
    }

    /// Compiles and runs `source`, letting `register` add extra natives (tests
    /// use this to observe handler scheduling from Rust).
    pub fn start_with(
        source_name: &str,
        source: &str,
        register: impl FnOnce(&mut ::dyon::Module) + Send + 'static,
    ) -> Result<Self, BridgeError> {
        let shared = Arc::new(BridgeShared::new());
        let (addr_tx, addr_rx) = sync_channel(1);
        {
            let mut guard = shared.addr_tx.lock().map_err(|_| BridgeError::Poisoned)?;
            *guard = Some(addr_tx);
        }

        let thread_shared = Arc::clone(&shared);
        let name = source_name.to_owned();
        let source = source.to_owned();
        let thread = thread::Builder::new()
            .name("vr-web-dyon".to_owned())
            .spawn(move || run_runtime(&name, &source, thread_shared, register))?;

        let addr = match addr_rx.recv() {
            Ok(Ok(addr)) => addr,
            Ok(Err(error)) => return Err(error),
            Err(_) => return Err(BridgeError::ServerNotStarted),
        };
        Ok(Self {
            shared,
            addr,
            thread: Some(thread),
        })
    }

    /// Compiles and runs `source` with the web natives **and** the SQLite
    /// natives from `vr-db`, so a handler can call `open_database`,
    /// `database_query` and the rest of the family.
    ///
    /// A database handle is a shared Dyon custom object (`Arc<Mutex<..>>`), so
    /// passing it from one native call to the next inside a handler works as
    /// in a desktop script. Handler calls still run one at a time on the
    /// runtime thread, so two requests never touch the same handle at once; a
    /// slow query blocks that thread — and every other request — for its
    /// duration. That head-of-line blocking is acceptable while handlers are
    /// expected to be quick, and is the price of keeping Dyon single-threaded.
    #[cfg(feature = "sqlite")]
    pub fn start_with_sqlite(source_name: &str, source: &str) -> Result<Self, BridgeError> {
        Self::start_with(source_name, source, ::vr_db::register)
    }

    /// The address the Dyon program bound.
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Requests shutdown. Dyon finishes the in-flight requests first.
    pub fn shutdown(&self) {
        self.shared.shutdown.trigger();
    }

    /// The most recent malformed-response or call error, if any.
    pub fn last_error(&self) -> Option<DyonResponseError> {
        self.shared.last_error()
    }

    /// Waits for the runtime thread to finish and returns its outcome.
    pub fn join(mut self) -> Result<(), BridgeError> {
        match self.thread.take() {
            Some(thread) => thread.join().map_err(|_| BridgeError::ThreadPanicked)?,
            None => Ok(()),
        }
    }
}

/// Runs on the dedicated runtime thread: compile, then run `main`, which is
/// expected to block in `server_start`.
fn run_runtime(
    source_name: &str,
    source: &str,
    shared: Arc<BridgeShared>,
    register: impl FnOnce(&mut ::dyon::Module) + Send + 'static,
) -> Result<(), BridgeError> {
    install(Arc::clone(&shared));
    let mut runtime = match DyonRuntime::from_source_with(source_name, source, move |module| {
        super::register(module);
        register(module);
    }) {
        Ok(runtime) => runtime,
        Err(error) => {
            shared.publish_addr(Err(BridgeError::Program(error.to_string())));
            return Err(BridgeError::Dyon(error));
        }
    };
    match runtime.run() {
        // If the address was already published, the server ran and stopped; if
        // not, `main` never called `server_start`.
        Ok(()) => {
            shared.publish_addr(Err(BridgeError::ServerNotStarted));
            Ok(())
        }
        Err(error) => {
            shared.publish_addr(Err(BridgeError::Program(error.to_string())));
            Err(BridgeError::Dyon(error))
        }
    }
}

/// Called by the `server_start` native. Starts the HTTP workers, then services
/// [`Call`]s on the calling thread until the last handler sender drops.
pub(crate) fn serve(
    runtime: &mut ::dyon::Runtime,
    routes: Vec<RouteSpec>,
    shutdown: Shutdown,
) -> Result<(), BridgeError> {
    if routes.is_empty() {
        return Err(BridgeError::NoRoutes);
    }

    let (calls_tx, calls_rx) = channel::<Call>();
    let mut router = Router::new();
    let mut registered = HashSet::new();
    for route in &routes {
        if registered.insert(route.handler.clone()) {
            router.register_handler(
                route.handler.clone(),
                DyonCallHandler {
                    function: route.handler.clone(),
                    calls: calls_tx.clone(),
                },
            )?;
        }
        router.route(route.method, route.pattern.clone(), route.handler.clone())?;
    }

    let server = Server::new(router, POOL_SIZE).bind(BIND_ADDR)?;
    let addr = server.local_addr()?;
    let http = thread::Builder::new()
        .name("vr-web-http".to_owned())
        .spawn(move || server.run_until(shutdown))?;

    // Only tell the host the address once the workers are actually running, so
    // a spawn failure surfaces as a startup error instead of a dead address.
    if let Some(state) = current() {
        state.publish_addr(Ok(addr));
    }

    // Dropping our own sender means the loop below ends exactly when every
    // worker handler (and thus the server) has been dropped.
    drop(calls_tx);
    service_calls(runtime, &calls_rx);

    http.join().map_err(|_| BridgeError::ThreadPanicked)??;
    Ok(())
}

/// The serialisation point: one Dyon call at a time, never nested.
fn service_calls(runtime: &mut ::dyon::Runtime, calls: &Receiver<Call>) {
    while let Ok(call) = calls.recv() {
        let response = dispatch(runtime, &call.function, &call.request);
        let _ = call.reply.send(response);
    }
}

fn dispatch(runtime: &mut ::dyon::Runtime, function: &str, request: &Request) -> Response {
    // A failed Dyon call can leave the value stacks partially grown (Dyon
    // truncates them on the success path only), so restore the exact lengths
    // around every call to keep handlers isolated from each other.
    let stack = runtime.stack.len();
    let locals = runtime.local_stack.len();
    let current = runtime.current_stack.len();
    let module = runtime.module.clone();

    let result = runtime.call_str_ret(function, &[request_object(request)], &module);

    runtime.stack.truncate(stack);
    runtime.local_stack.truncate(locals);
    runtime.current_stack.truncate(current);

    match result {
        Ok(value) => match response_from_variable(&value) {
            Ok(response) => response,
            Err(error) => {
                record_error(error.clone());
                Response::text(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("the handler returned an invalid response: {error}"),
                )
            }
        },
        Err(message) => {
            record_error(DyonResponseError::Call(message.clone()));
            Response::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("the handler `{function}` failed: {message}"),
            )
        }
    }
}

fn record_error(error: DyonResponseError) {
    if let Some(state) = current() {
        state.record(error);
    }
}
