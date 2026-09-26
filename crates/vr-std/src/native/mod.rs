//! Dyon bindings for the standard library.
//!
//! [`register`] is the single entry point the runtime calls before loading
//! source, so Dyon's lifetime checker sees every signature. Each command family
//! lives in its own submodule to keep the files small; the wrappers pop their
//! arguments in reverse declaration order (Dyon pushes them left to right) and
//! render typed errors as the only error a Dyon native may return, a `String`.

use dyon::Module;

mod encoding;
mod hash;
mod math;
mod regex;
mod strings;

/// Registers every standard-library command into `module`.
///
/// Must run before the source is loaded so Dyon's lifetime checker sees the
/// signatures.
pub fn register(module: &mut Module) {
    strings::register(module);
    encoding::register(module);
    regex::register(module);
    hash::register(module);
    math::register(module);
}
