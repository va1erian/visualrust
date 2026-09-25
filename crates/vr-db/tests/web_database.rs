//! A real HTTP handler reads SQLite through the runtime bridge.
//!
//! These tests live in `vr-db` rather than `vr-web` because they need both
//! crates at once: `vr-web`'s `sqlite` feature pulls `vr-db` in, so making
//! `vr-db` dev-depend on `vr-web` keeps the production graph acyclic while
//! still letting `cargo test -p vr-db` exercise the assembled runtime.

mod common;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::thread;
use std::time::Duration;

use common::TempDb;
use vr_db::{Database, DbValue};
use vr_web::WebRuntime;

/// Fills `people` with `(id, name)` rows so a handler has data to read back.
fn populate(path: &Path, rows: &[(i64, &str)]) {
    let mut database = Database::open(path).expect("open temp database");
    database
        .exec(
            "CREATE TABLE people (id INTEGER PRIMARY KEY, name TEXT)",
            &[],
        )
        .expect("create table");
    for (id, name) in rows {
        database
            .exec(
                "INSERT INTO people (id, name) VALUES (?, ?)",
                &[DbValue::Integer(*id), DbValue::Text((*name).to_owned())],
            )
            .expect("insert row");
    }
}

/// The Dyon program: two routes that open the temp database and read from it.
fn script(path: &Path) -> String {
    // Forward slashes keep the path a plain Dyon string literal on Windows and
    // are accepted by SQLite there too.
    let path = path.to_string_lossy().replace('\\', "/");
    format!(
        r#"
fn person(req: {{}}) -> {{}} {{
    db := open_database("{path}")
    n := database_query(db, "SELECT name FROM people WHERE id = 1", [])
    body := "missing"
    if n > 0 {{
        if database_next_row(db) {{
            body = str(database_column(db, 0))
        }}
    }}
    // `clone` detaches the local from the function's argument lifetime, which
    // the checker otherwise rejects when the value lands in the returned object.
    return {{status: 200, body: clone(body), content_type: "text/plain"}}
}}

fn people(req: {{}}) -> {{}} {{
    db := open_database("{path}")
    count := database_query(db, "SELECT name FROM people ORDER BY id", [])
    body := ""
    for i count {{
        if database_next_row(db) {{
            body += str(database_column(db, 0)) + " "
        }}
    }}
    return {{status: 200, body: clone(body), content_type: "text/plain"}}
}}

fn main() {{
    server := server_new()
    server_route(server, "GET", "/person", "person")
    server_route(server, "GET", "/people", "people")
    server_start(server)
}}
"#
    )
}

/// Sends a minimal `GET` and reads the whole response until the server closes.
fn get(addr: SocketAddr, path: &str) -> String {
    let mut stream = TcpStream::connect(addr).expect("connect to server");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set read timeout");
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .expect("write request");
    stream.flush().expect("flush request");
    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("read response");
    String::from_utf8_lossy(&response).into_owned()
}

/// The bytes after the header terminator of a raw HTTP response.
fn http_body(response: &str) -> &str {
    response.split_once("\r\n\r\n").map_or("", |(_, body)| body)
}

#[test]
fn handler_returns_a_row_read_from_sqlite() {
    let temp = TempDb::new("web-row");
    // A value that appears nowhere in the script, so a matching body can only
    // have come from the database.
    let marker = format!("row-{}", std::process::id());
    populate(temp.path(), &[(1, marker.as_str()), (2, "Bob")]);

    let runtime = WebRuntime::start_with_sqlite("web_db_row.dyon", &script(temp.path()))
        .expect("server starts");
    let response = get(runtime.local_addr(), "/person");
    runtime.shutdown();
    runtime.join().expect("the runtime joins");

    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert_eq!(http_body(&response), marker, "{response}");
}

#[test]
fn concurrent_requests_read_the_database_through_the_bridge() {
    let temp = TempDb::new("web-concurrent");
    let marker = format!("row-{}", std::process::id());
    populate(temp.path(), &[(1, marker.as_str()), (2, "Bob")]);
    let expected = format!("{marker} Bob ");

    let runtime = WebRuntime::start_with_sqlite("web_db_concurrent.dyon", &script(temp.path()))
        .expect("server starts");
    let addr = runtime.local_addr();

    let clients: Vec<_> = (0..2)
        .map(|_| {
            let expected = expected.clone();
            thread::spawn(move || {
                let response = get(addr, "/people");
                assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
                assert_eq!(http_body(&response), expected, "{response}");
            })
        })
        .collect();
    for client in clients {
        client.join().expect("client thread");
    }

    runtime.shutdown();
    runtime.join().expect("the runtime joins");
}
