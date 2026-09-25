//! Concurrency smoke test: many sockets hit the server at once, but the Dyon
//! runtime thread must service the handlers strictly one at a time.
//!
//! A probe native records how many handlers are "inside" at once; if the bridge
//! ever called a handler re-entrantly the recorded maximum would exceed one.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use common::get;
use dyon::{Dfn, Module, Runtime, Type, Variable};
use vr_web::WebRuntime;

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static MAX_ACTIVE: AtomicUsize = AtomicUsize::new(0);

fn vr_test_begin(_runtime: &mut Runtime) -> Result<(), String> {
    let now = ACTIVE.fetch_add(1, Ordering::SeqCst) + 1;
    MAX_ACTIVE.fetch_max(now, Ordering::SeqCst);
    // Widening the window makes a re-entrant call observable.
    thread::sleep(std::time::Duration::from_millis(25));
    Ok(())
}

fn vr_test_end(_runtime: &mut Runtime) -> Result<(), String> {
    ACTIVE.fetch_sub(1, Ordering::SeqCst);
    Ok(())
}

fn vr_test_summary(_runtime: &mut Runtime) -> Result<Variable, String> {
    let max = MAX_ACTIVE.load(Ordering::SeqCst);
    let text = if max <= 1 { "serial" } else { "reentered" };
    Ok(Variable::Str(Arc::new(text.to_owned())))
}

fn register_probe(module: &mut Module) {
    module.add_str("vr_test_begin", vr_test_begin, Dfn::nl(vec![], Type::Void));
    module.add_str("vr_test_end", vr_test_end, Dfn::nl(vec![], Type::Void));
    module.add_str(
        "vr_test_summary",
        vr_test_summary,
        Dfn::nl(vec![], Type::Str),
    );
}

const SCRIPT: &str = r#"
fn work(req: {}) -> {} {
    vr_test_begin()
    vr_test_end()
    return {status: 200, body: "ok", content_type: "text/plain"}
}

fn probe(req: {}) -> {} {
    return {status: 200, body: vr_test_summary(), content_type: "text/plain"}
}

fn main() {
    server := server_new()
    server_route(server, "GET", "/work", "work")
    server_route(server, "GET", "/probe", "probe")
    server_start(server)
}
"#;

#[test]
fn serialises_handlers_across_concurrent_clients() {
    ACTIVE.store(0, Ordering::SeqCst);
    MAX_ACTIVE.store(0, Ordering::SeqCst);

    let runtime =
        WebRuntime::start_with("concurrency.dyon", SCRIPT, register_probe).expect("server starts");
    let addr = runtime.local_addr();

    let clients: Vec<_> = (0..16)
        .map(|_| {
            thread::spawn(move || {
                let response = get(addr, "/work");
                assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
            })
        })
        .collect();
    for client in clients {
        client.join().expect("client thread");
    }

    let probe = get(addr, "/probe");
    assert!(probe.ends_with("serial"), "handlers overlapped: {probe}");

    runtime.shutdown();
    runtime.join().expect("the runtime joins");
}
