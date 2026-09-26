//! Embeds `vr-ide.rc` (the Common Controls v6 + per-monitor-v2 DPI manifest)
//! into the `vr-ide` binary and its test binaries.
//!
//! The toolbar and status bar are real child windows and the window itself is a
//! xui app window, so the common-controls classes must load through the v6
//! manifest. This mirrors `crates/vr-dyon/build.rs` and `xui/build.rs`; the
//! resource only makes sense for an executable, and on non-Windows hosts the
//! whole step is a no-op.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    println!("cargo:rerun-if-changed=vr-ide.rc");
    println!("cargo:rerun-if-changed=vr-ide.manifest");

    embed_resource::compile_for_everything("vr-ide.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("compile the vr-ide manifest");
}
