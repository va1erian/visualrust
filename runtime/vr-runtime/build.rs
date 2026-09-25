//! Embeds the Common Controls v6 manifest into `vr-runtime`'s test binaries, the
//! same way `vr-dyon` and `vr-ide` do for theirs: the hello-window sample builds
//! real win32ui controls through the `ui_*` bindings, and without the manifest
//! `InitCommonControlsEx` fails and no control can be created.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    println!("cargo:rerun-if-changed=vr-runtime.rc");
    println!("cargo:rerun-if-changed=vr-runtime.manifest");

    embed_resource::compile_for_tests("vr-runtime.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("compile the vr-runtime test manifest");
}
