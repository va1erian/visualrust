//! End-to-end: a Dyon script registers two routes and serves them over a real
//! ephemeral TCP port. The response bodies are computed inside Dyon, so a green
//! assertion proves the request reached a Dyon handler and came back.

mod common;

use common::get;
use vr_web::WebRuntime;

const SCRIPT: &str = r#"
fn index(req: {}) -> {} {
    return {status: 200, body: "index from dyon", content_type: "text/plain"}
}

fn hello(req: {}) -> {} {
    name := req.params.name
    return {status: 200, body: "hey " + name, content_type: "text/plain"}
}

fn main() {
    server := server_new()
    server_route(server, "GET", "/", "index")
    server_route(server, "GET", "/hello/:name", "hello")
    server_start(server)
}
"#;

#[test]
fn serves_two_dyon_handlers() {
    let runtime = WebRuntime::start("roundtrip.dyon", SCRIPT).expect("the server starts");
    let addr = runtime.local_addr();

    let index = get(addr, "/");
    assert!(index.starts_with("HTTP/1.1 200 OK\r\n"), "{index}");
    assert!(index.ends_with("index from dyon"), "{index}");

    let hello = get(addr, "/hello/ada");
    assert!(hello.starts_with("HTTP/1.1 200 OK\r\n"), "{hello}");
    assert!(hello.ends_with("hey ada"), "{hello}");

    runtime.shutdown();
    runtime.join().expect("the runtime joins");
}

#[test]
fn unknown_paths_still_get_the_http_404() {
    let runtime = WebRuntime::start("roundtrip.dyon", SCRIPT).expect("the server starts");
    let response = get(runtime.local_addr(), "/missing");
    assert!(
        response.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "{response}"
    );
    runtime.shutdown();
    runtime.join().expect("the runtime joins");
}
