//! End-to-end smoke test for the `examples/web-api` sample (#62).
//!
//! The test treats the sample as the source of truth: it loads the project
//! manifest through `vr-core`, checks the declared routes match the
//! `server_route` registrations in `src/main.dyon`, swaps the sample's default
//! database file for a temp path, then starts the real server through
//! [`WebRuntime::start_with_sqlite`] on an ephemeral port.
//!
//! It then drives the server with genuine `TcpStream` requests: create a row,
//! GET it back, and assert the response came from SQLite. The runtime is shut
//! down and joined on every exit path, so no server is left running.
//!
//! Gated on the `sqlite` feature because the default `vr-web` build carries no
//! SQLite; the sample needs the database natives.

#![cfg(feature = "sqlite")]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use vr_core::manifest::{Manifest, ProjectKind};
use vr_web::WebRuntime;

/// The default database file named in the sample. The test replaces this exact
/// literal with a temp path so the sample's committed source stays runnable
/// while the test never touches real data.
const DB_PLACEHOLDER: &str = "web-api.sqlite";

/// A temp SQLite file that removes itself, mirroring the `vr-db` test helper.
struct TempDb {
    path: PathBuf,
}

impl TempDb {
    fn new(tag: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("vr-web-api-{tag}-{}.sqlite", std::process::id()));
        let _ = std::fs::remove_file(&path);
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// The `examples/web-api` directory, anchored to this crate's manifest.
fn sample_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("web-api")
}

/// Reads the sample entry point and points its database at `db`.
///
/// The replacement is asserted so a renamed literal fails loudly instead of
/// silently running against the committed file.
fn sample_source(entry: &Path, db: &Path) -> String {
    let source = std::fs::read_to_string(entry).expect("the sample entry point is readable");
    assert!(
        source.contains(DB_PLACEHOLDER),
        "the sample must name `{DB_PLACEHOLDER}` so the test can redirect it"
    );
    // Forward slashes keep the path a plain Dyon string literal on Windows and
    // are accepted by SQLite there too.
    let db = db.to_string_lossy().replace('\\', "/");
    source.replace(DB_PLACEHOLDER, &db)
}

/// Sends raw bytes and reads until the server closes the connection.
fn raw_request(addr: SocketAddr, raw: &str) -> String {
    let mut stream = TcpStream::connect(addr).expect("connect to server");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set read timeout");
    stream.write_all(raw.as_bytes()).expect("write request");
    stream.flush().expect("flush request");
    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("read response");
    String::from_utf8_lossy(&response).into_owned()
}

/// A minimal `GET` with `Connection: close`.
fn get(addr: SocketAddr, path: &str) -> String {
    raw_request(
        addr,
        &format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"),
    )
}

/// A minimal `POST` with a plain-text body and `Connection: close`.
fn post(addr: SocketAddr, path: &str, body: &str) -> String {
    raw_request(
        addr,
        &format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: text/plain\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ),
    )
}

/// The bytes after the header terminator of a raw HTTP response.
fn http_body(response: &str) -> &str {
    response.split_once("\r\n\r\n").map_or("", |(_, body)| body)
}

#[test]
fn sample_creates_a_row_then_reads_it_back() {
    let project = sample_dir();
    let manifest = Manifest::load(project.join("vrproj.toml")).expect("the sample manifest loads");
    assert_eq!(
        manifest.project.kind,
        ProjectKind::Web,
        "the sample is a web project"
    );
    assert!(!manifest.routes.is_empty(), "the sample declares routes");
    assert!(manifest.db.is_some(), "the sample declares a database");
    assert_eq!(
        manifest
            .db
            .as_ref()
            .map(|db| db.path.to_string_lossy().into_owned()),
        Some(DB_PLACEHOLDER.to_owned()),
        "the manifest database path matches the placeholder"
    );

    let entry = project.join(&manifest.project.entry);
    let source = std::fs::read_to_string(&entry).expect("the sample entry point is readable");

    // The manifest is the source of truth for the API surface; a route that is
    // declared but not registered in Dyon would never be served.
    for route in &manifest.routes {
        let registration = format!(
            "server_route(server, \"{}\", \"{}\", \"{}\")",
            route.method, route.path, route.handler
        );
        assert!(
            source.contains(&registration),
            "the sample registers `{registration}`"
        );
    }

    let temp = TempDb::new("roundtrip");
    let source = sample_source(&entry, temp.path());
    let runtime = WebRuntime::start_with_sqlite("web_api_sample.dyon", &source)
        .expect("the sample server starts");
    let addr = runtime.local_addr();

    // A value that appears nowhere in the script, so a matching body can only
    // have come from the SQLite row we just inserted.
    let marker = format!("note-{}", std::process::id());

    let created = post(addr, "/notes", &marker);
    assert!(created.starts_with("HTTP/1.1 201 "), "{created}");
    let created_json: serde_json::Value =
        serde_json::from_str(http_body(&created)).expect("the create response is JSON");
    let id = created_json
        .get("id")
        .and_then(serde_json::Value::as_u64)
        .expect("the create response carries an integer id");
    assert_eq!(
        created_json.get("body").and_then(serde_json::Value::as_str),
        Some(marker.as_str()),
        "{created}"
    );

    let fetched = get(addr, &format!("/notes/{id}"));
    assert!(fetched.starts_with("HTTP/1.1 200 OK\r\n"), "{fetched}");
    let fetched_json: serde_json::Value =
        serde_json::from_str(http_body(&fetched)).expect("the read response is JSON");
    assert_eq!(
        fetched_json.get("id").and_then(serde_json::Value::as_u64),
        Some(id),
        "{fetched}"
    );
    assert_eq!(
        fetched_json.get("body").and_then(serde_json::Value::as_str),
        Some(marker.as_str()),
        "the row read back must be the one just written: {fetched}"
    );

    // The list route must also surface the persisted row.
    let listed = get(addr, "/notes");
    assert!(listed.starts_with("HTTP/1.1 200 OK\r\n"), "{listed}");
    assert!(
        http_body(&listed).contains(&marker),
        "the list response contains the created note: {listed}"
    );

    runtime.shutdown();
    runtime.join().expect("the runtime joins");
}

#[test]
fn missing_note_is_a_json_404() {
    let project = sample_dir();
    let manifest = Manifest::load(project.join("vrproj.toml")).expect("the sample manifest loads");
    let entry = project.join(&manifest.project.entry);

    let temp = TempDb::new("not-found");
    let source = sample_source(&entry, temp.path());
    let runtime = WebRuntime::start_with_sqlite("web_api_sample.dyon", &source)
        .expect("the sample server starts");
    let addr = runtime.local_addr();

    let missing = get(addr, "/notes/999999");
    runtime.shutdown();
    runtime.join().expect("the runtime joins");

    assert!(
        missing.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "{missing}"
    );
    assert!(
        http_body(&missing).contains("not found"),
        "the 404 carries a JSON error: {missing}"
    );
}
