//! Error paths: a handler that raises, or returns something other than the
//! documented response object, becomes a `500` and never takes the server down.

mod common;

use common::get;
use vr_web::{DyonResponseError, WebRuntime};

const SCRIPT: &str = r#"
fn boom(req: {}) -> {} {
    x := 1
    x = "two"
    return {status: 200, body: "no", content_type: "text/plain"}
}

fn garbage(req: {}) -> f64 {
    return 123
}

fn main() {
    server := server_new()
    server_route(server, "GET", "/boom", "boom")
    server_route(server, "GET", "/garbage", "garbage")
    server_start(server)
}
"#;

#[test]
fn a_raising_handler_is_a_500() {
    let runtime = WebRuntime::start("errors.dyon", SCRIPT).expect("server starts");
    let response = get(runtime.local_addr(), "/boom");
    assert!(
        response.starts_with("HTTP/1.1 500 Internal Server Error\r\n"),
        "{response}"
    );
    assert!(
        matches!(runtime.last_error(), Some(DyonResponseError::Call(_))),
        "expected a typed call error, got {:?}",
        runtime.last_error()
    );
    runtime.shutdown();
    runtime.join().expect("the runtime joins");
}

#[test]
fn a_malformed_return_is_a_500_and_the_server_survives() {
    let runtime = WebRuntime::start("errors.dyon", SCRIPT).expect("server starts");
    let addr = runtime.local_addr();

    let garbage = get(addr, "/garbage");
    assert!(
        garbage.starts_with("HTTP/1.1 500 Internal Server Error\r\n"),
        "{garbage}"
    );
    assert!(
        matches!(
            runtime.last_error(),
            Some(DyonResponseError::NotAnObject { .. })
        ),
        "expected a typed mapping error, got {:?}",
        runtime.last_error()
    );

    // The runtime thread must still be alive after mapping a bad response.
    let again = get(addr, "/garbage");
    assert!(
        again.starts_with("HTTP/1.1 500 Internal Server Error\r\n"),
        "{again}"
    );

    runtime.shutdown();
    runtime.join().expect("the runtime joins");
}
