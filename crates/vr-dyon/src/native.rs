//! Native functions exposed to Dyon programs.
//!
//! This is the seam `ui_*` bindings will plug into; the two functions here
//! exist to prove registration and value round-tripping work end to end.

use dyon::{Dfn, Module, Type, dyon_fn, dyon_fn_pop, dyon_macro_items};

dyon_fn! {fn vr_native_add(a: f64, b: f64) -> f64 {
    a + b
}}

dyon_fn! {fn vr_native_echo(text: String) -> String {
    text
}}

/// Registers every native function into `module`.
///
/// Must run before source is loaded so the lifetime checker sees the
/// signatures.
pub(crate) fn register(module: &mut Module) {
    module.add_str(
        "vr_native_add",
        vr_native_add,
        Dfn::nl(vec![Type::F64, Type::F64], Type::F64),
    );
    module.add_str(
        "vr_native_echo",
        vr_native_echo,
        Dfn::nl(vec![Type::Str], Type::Str),
    );
}
