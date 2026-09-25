//! The blocking server: a bounded worker pool and graceful shutdown.

use std::io::{BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use thiserror::Error;

use crate::handler::Handler;
use crate::parse::{ParseLimits, parse_request};
use crate::request::Request;
use crate::response::{Response, StatusCode};
use crate::router::{Resolution, Router};

/// How long [`Server::run_until`] sleeps between non-blocking accepts.
///
/// A blocking `accept` cannot observe a shutdown flag on Windows without a
/// self-pipe we do not have; a short poll keeps shutdown prompt at negligible
/// cost.
const ACCEPT_POLL: Duration = Duration::from_millis(2);
/// Per-socket read/write timeout so one stalled client cannot pin a worker.
const IO_TIMEOUT: Duration = Duration::from_secs(5);
/// Per-request head and body caps, forwarded to the parser.
const MAX_HEAD_BYTES: usize = 64 * 1024;
/// See [`MAX_HEAD_BYTES`].
const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;

/// Errors from binding or running the server.
#[derive(Debug, Error)]
pub enum ServerError {
    /// `run_until` or `local_addr` was called before `bind`.
    #[error("the server has no bound listener; call Server::bind first")]
    NotBound,
    /// The listener or a socket failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// The worker pool stopped accepting jobs.
    #[error("the worker pool is no longer accepting connections")]
    PoolClosed,
}

/// A clonable flag used to stop the accept loop.
///
/// Cloning shares the flag, so a test thread can trigger shutdown while the
/// server owns another clone.
#[derive(Clone, Default)]
pub struct Shutdown {
    triggered: Arc<AtomicBool>,
}

impl Shutdown {
    /// A fresh, untriggered flag.
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests shutdown. The accept loop notices within one poll interval.
    pub fn trigger(&self) {
        self.triggered.store(true, Ordering::SeqCst);
    }

    /// Whether shutdown has been requested.
    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::SeqCst)
    }
}

/// A worker-pool request.
struct Job(TcpStream);

/// A blocking HTTP/1.1 server over a router.
///
/// Construction and binding are separate so the caller can learn the ephemeral
/// port from `local_addr` (tests bind `127.0.0.1:0`) before `run_until` blocks.
pub struct Server {
    router: Arc<Router>,
    pool_size: usize,
    listener: Option<TcpListener>,
}

impl Server {
    /// Builds a server around `router`. `pool_size` is clamped to at least one
    /// worker so the queue can never deadlock with no consumer.
    pub fn new(router: Router, pool_size: usize) -> Self {
        Self {
            router: Arc::new(router),
            pool_size: pool_size.max(1),
            listener: None,
        }
    }

    /// Binds a TCP listener. Use `127.0.0.1:0` for an ephemeral port.
    pub fn bind(mut self, addr: impl ToSocketAddrs) -> Result<Self, ServerError> {
        self.listener = Some(TcpListener::bind(addr)?);
        Ok(self)
    }

    /// The bound address.
    pub fn local_addr(&self) -> Result<SocketAddr, ServerError> {
        self.listener
            .as_ref()
            .ok_or(ServerError::NotBound)?
            .local_addr()
            .map_err(ServerError::from)
    }

    /// Serves requests until `shutdown` is triggered.
    ///
    /// Each connection handles exactly one request and is then closed
    /// (`connection: close`). When shutdown fires, the listener stops accepting,
    /// the queue is drained and the workers are joined, so in-flight requests
    /// finish before this returns.
    pub fn run_until(self, shutdown: Shutdown) -> Result<(), ServerError> {
        let Self {
            router,
            pool_size,
            listener,
        } = self;
        let listener = listener.ok_or(ServerError::NotBound)?;
        listener.set_nonblocking(true)?;

        let (sender, receiver) = sync_channel::<Job>(pool_size * 4);
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers: Vec<JoinHandle<()>> = Vec::with_capacity(pool_size);
        for _ in 0..pool_size {
            let receiver = Arc::clone(&receiver);
            let router = Arc::clone(&router);
            let worker = thread::Builder::new()
                .name("vr-web-worker".to_owned())
                .spawn(move || worker_loop(&receiver, &router))?;
            workers.push(worker);
        }

        let mut outcome = Ok(());
        while !shutdown.is_triggered() {
            match listener.accept() {
                Ok((stream, _peer)) => {
                    if let Err(error) = enqueue(&sender, stream) {
                        outcome = Err(error);
                        break;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(ACCEPT_POLL);
                }
                Err(error) => {
                    outcome = Err(ServerError::Io(error));
                    break;
                }
            }
        }

        // Dropping the sender ends every worker's `recv`, draining the queue.
        drop(sender);
        for worker in workers {
            let _ = worker.join();
        }
        outcome
    }
}

fn enqueue(sender: &SyncSender<Job>, stream: TcpStream) -> Result<(), ServerError> {
    match sender.try_send(Job(stream)) {
        Ok(()) => Ok(()),
        // The queue is bounded; block rather than drop the connection.
        Err(TrySendError::Full(job)) => sender.send(job).map_err(|_| ServerError::PoolClosed),
        Err(TrySendError::Disconnected(_)) => Err(ServerError::PoolClosed),
    }
}

fn worker_loop(receiver: &Arc<Mutex<Receiver<Job>>>, router: &Router) {
    loop {
        // The guard is dropped before `serve`, so workers only serialise while
        // waiting for a job, not while handling one.
        let job = match receiver.lock() {
            Ok(guard) => guard.recv(),
            Err(_) => return,
        };
        match job {
            Ok(Job(stream)) => serve(stream, router),
            Err(_) => return,
        }
    }
}

fn serve(stream: TcpStream, router: &Router) {
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
    let mut reader = BufReader::new(stream);
    let limits = ParseLimits {
        max_head: MAX_HEAD_BYTES,
        max_body: MAX_BODY_BYTES,
    };
    let response = match parse_request(&mut reader, &limits) {
        Ok(Some(request)) => dispatch(router, request),
        Ok(None) => return,
        Err(error) => Response::text(StatusCode::BAD_REQUEST, format!("bad request: {error}")),
    };
    let bytes = response.to_http();
    let mut stream = reader.into_inner();
    let _ = stream.write_all(&bytes);
    let _ = stream.flush();
}

fn dispatch(router: &Router, mut request: Request) -> Response {
    match router.resolve(request.method, &request.path) {
        Resolution::Matched(matched) => {
            request.params = matched.params;
            match router.handler(&matched.handler) {
                Some(handler) => run_handler(&*handler, &request),
                None => Response::text(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("handler `{}` is not registered", matched.handler),
                ),
            }
        }
        Resolution::NotFound => Response::text(StatusCode::NOT_FOUND, "not found"),
        Resolution::MethodNotAllowed { allowed } => {
            let allow = allowed
                .iter()
                .map(|method| method.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            Response::text(StatusCode::METHOD_NOT_ALLOWED, "method not allowed")
                .with_header("allow", allow)
        }
    }
}

/// A user handler must not be able to take a worker down with it, so a panic
/// becomes a `500` and the pool keeps serving.
fn run_handler(handler: &dyn Handler, request: &Request) -> Response {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler.handle(request))) {
        Ok(response) => response,
        Err(_) => Response::text(StatusCode::INTERNAL_SERVER_ERROR, "handler panicked"),
    }
}

#[cfg(test)]
mod tests {
    use super::{Server, ServerError, Shutdown};

    #[test]
    fn shutdown_flag_is_shared_between_clones() {
        let shutdown = Shutdown::new();
        let clone = shutdown.clone();
        assert!(!clone.is_triggered());
        shutdown.trigger();
        assert!(clone.is_triggered());
    }

    #[test]
    fn requires_a_bind_before_running() {
        let server = Server::new(crate::Router::new(), 2);
        assert!(matches!(server.local_addr(), Err(ServerError::NotBound)));
    }

    #[test]
    fn pool_size_is_clamped_to_one() {
        let server = Server::new(crate::Router::new(), 0);
        assert_eq!(server.pool_size, 1);
    }
}
