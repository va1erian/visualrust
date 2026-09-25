//! Embeds the Common Controls v6 manifest into `vr-dyon`'s test binaries, the
//! same way win32ui does for its own: the `ui_*` bindings create real win32ui
//! controls, and without the manifest `InitCommonControlsEx` fails.

fn main() {
    // The manifest only matters for the `ui_*` test binaries; a build with the
    // `ui` feature off has no controls to register and must not pull the
    // resource compiler into a headless (web-only) dependency graph.
    if std::env::var("CARGO_FEATURE_UI").is_err() {
        return;
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    println!("cargo:rerun-if-changed=vr-dyon.rc");
    println!("cargo:rerun-if-changed=vr-dyon.manifest");

    embed_resource::compile_for_tests("vr-dyon.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("compile the vr-dyon test manifest");
}
