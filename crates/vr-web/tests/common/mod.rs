//! Shared helpers for the `vr-web` integration tests.
//!
//! Each integration test binary compiles this module but uses only part of it,
//! so dead-code analysis is silenced rather than duplicated helpers left in
//! every file.
#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use vr_web::{Router, Server, ServerError, Shutdown};

/// Binds an ephemeral port, starts the server on a background thread, and
/// returns the address, a shutdown handle and the join handle.
pub fn spawn_server(
    router: Router,
    pool_size: usize,
) -> (SocketAddr, Shutdown, JoinHandle<Result<(), ServerError>>) {
    let server = Server::new(router, pool_size)
        .bind("127.0.0.1:0")
        .expect("bind ephemeral port");
    let addr = server.local_addr().expect("read local addr");
    let shutdown = Shutdown::new();
    let signal = shutdown.clone();
    let handle = thread::spawn(move || server.run_until(signal));
    (addr, shutdown, handle)
}

/// Sends raw bytes and reads until the server closes the connection.
pub fn raw_request(addr: SocketAddr, raw: &str) -> String {
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

/// A minimal `GET` request with `Connection: close`.
pub fn get(addr: SocketAddr, path: &str) -> String {
    raw_request(
        addr,
        &format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"),
    )
}
