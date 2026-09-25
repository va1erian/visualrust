//! Concurrency smoke test: many clients hit a small worker pool at once.

mod common;

use common::{get, spawn_server};
use std::thread;
use std::time::Duration;
use vr_web::{Method, Request, Response, Router, StatusCode};

fn slow_router() -> Router {
    let mut router = Router::new();
    router
        .route(Method::Get, "/slow/:n", "slow")
        .expect("route");
    router
        .register_handler("slow", |request: &Request| {
            // Sleeping inside the handler forces requests to overlap, so the
            // test exercises the pool rather than serialised round-trips.
            thread::sleep(Duration::from_millis(20));
            let n = request.param("n").unwrap_or("?");
            Response::text(StatusCode::OK, format!("done {n}"))
        })
        .expect("handler");
    router
}

#[test]
fn handles_concurrent_clients() {
    let (addr, shutdown, handle) = spawn_server(slow_router(), 4);
    let clients: Vec<_> = (0..16)
        .map(|n| {
            thread::spawn(move || {
                let response = get(addr, &format!("/slow/{n}"));
                assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
                assert!(response.ends_with(&format!("done {n}")), "{response}");
            })
        })
        .collect();
    for client in clients {
        client.join().expect("client thread");
    }
    shutdown.trigger();
    handle.join().expect("join").expect("serve");
}
