//! End-to-end request round-trips over a real ephemeral TCP port.

mod common;

use common::{get, raw_request, spawn_server};
use vr_web::{Method, Request, Response, Router, StatusCode};

fn router() -> Router {
    let mut router = Router::new();
    router
        .route(Method::Get, "/hello/:name", "hello")
        .expect("route");
    router.route(Method::Post, "/echo", "echo").expect("route");
    router
        .register_handler("hello", |request: &Request| {
            let name = request.param("name").unwrap_or("world");
            let greeting = request.query("greet").unwrap_or("hi");
            Response::text(StatusCode::OK, format!("{greeting} {name}"))
        })
        .expect("handler");
    router
        .register_handler("echo", |request: &Request| {
            Response::new(StatusCode::OK).with_body(request.body.clone())
        })
        .expect("handler");
    router
}

#[test]
fn serves_a_matched_route_with_params_and_query() {
    let (addr, shutdown, handle) = spawn_server(router(), 2);
    let response = get(addr, "/hello/ada?greet=hey");
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.ends_with("hey ada"), "{response}");
    shutdown.trigger();
    handle.join().expect("join").expect("serve");
}

#[test]
fn serves_a_post_body() {
    let (addr, shutdown, handle) = spawn_server(router(), 2);
    let response = raw_request(
        addr,
        "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
    );
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.ends_with("hello"), "{response}");
    shutdown.trigger();
    handle.join().expect("join").expect("serve");
}

#[test]
fn returns_404_for_an_unknown_path() {
    let (addr, shutdown, handle) = spawn_server(router(), 2);
    let response = get(addr, "/missing");
    assert!(
        response.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "{response}"
    );
    shutdown.trigger();
    handle.join().expect("join").expect("serve");
}

#[test]
fn returns_405_with_an_allow_header_for_a_known_path() {
    let (addr, shutdown, handle) = spawn_server(router(), 2);
    let response = raw_request(
        addr,
        "DELETE /hello/ada HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert!(
        response.starts_with("HTTP/1.1 405 Method Not Allowed\r\n"),
        "{response}"
    );
    assert!(
        response.to_ascii_lowercase().contains("allow: get"),
        "{response}"
    );
    shutdown.trigger();
    handle.join().expect("join").expect("serve");
}

#[test]
fn returns_400_for_a_malformed_request() {
    let (addr, shutdown, handle) = spawn_server(router(), 2);
    let response = raw_request(addr, "NOT A REQUEST\r\n\r\n");
    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "{response}"
    );
    shutdown.trigger();
    handle.join().expect("join").expect("serve");
}
