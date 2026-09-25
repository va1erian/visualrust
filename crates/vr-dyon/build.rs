//! Embeds the Common Controls v6 manifest into `vr-dyon`'s test binaries, the
//! same way win32ui does for its own: the `ui_*` bindings create real win32ui
//! controls, and without the manifest `InitCommonControlsEx` fails.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    println!("cargo:rerun-if-changed=vr-dyon.rc");
    println!("cargo:rerun-if-changed=vr-dyon.manifest");

    embed_resource::compile_for_tests("vr-dyon.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("compile the vr-dyon test manifest");
}
