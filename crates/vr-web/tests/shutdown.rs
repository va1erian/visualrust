//! Graceful shutdown: `run_until` returns and joins its workers.

mod common;

use common::{get, spawn_server};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use vr_web::{Method, Response, Router, StatusCode};

fn router() -> Router {
    let mut router = Router::new();
    router.route(Method::Get, "/", "index").expect("route");
    router
        .register_handler("index", |_: &vr_web::Request| {
            Response::text(StatusCode::OK, "ok")
        })
        .expect("handler");
    router
}

#[test]
fn run_until_returns_after_shutdown() {
    let (addr, shutdown, handle) = spawn_server(router(), 2);
    assert!(get(addr, "/").starts_with("HTTP/1.1 200 OK\r\n"));

    shutdown.trigger();

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(handle.join());
    });
    match receiver.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(Ok(()))) => {}
        Ok(_) => panic!("server thread exited with an error or panicked"),
        Err(_) => panic!("server did not shut down within 10s"),
    }
}
