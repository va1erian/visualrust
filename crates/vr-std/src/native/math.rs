//! Dyon bindings for the math commands.
//!
//! Each wrapper pops its arguments in reverse declaration order (Dyon pushes
//! them left to right). The domain-checked commands render their
//! [`MathError`] as a Dyon error string; the rest cannot fail. Registering
//! these overrides Dyon's own prelude names (`sqrt`, `sin`, `min`, ...) so the
//! documented semantics win for a program that passed [`vr_std::register`].
//!
//! [`vr_std::register`]: crate::register

use dyon::{Dfn, Module, Runtime, Type, Variable};

use crate::error::MathError;
use crate::math;

/// Renders a value-level failure for Dyon.
fn dyon_error(error: MathError) -> String {
    error.to_string()
}

/// Pops one number and applies an infallible unary command.
fn unary(rt: &mut Runtime, function: fn(f64) -> f64) -> Result<Variable, String> {
    let value: f64 = rt.pop()?;
    Ok(Variable::f64(function(value)))
}

/// Pops one number and applies a domain-checked unary command.
fn checked_unary(
    rt: &mut Runtime,
    function: fn(f64) -> Result<f64, MathError>,
) -> Result<Variable, String> {
    let value: f64 = rt.pop()?;
    function(value).map(Variable::f64).map_err(dyon_error)
}

/// Pops two numbers and applies an infallible binary command.
///
/// The right operand is popped first because Dyon pushes arguments left to
/// right.
fn binary(rt: &mut Runtime, function: fn(f64, f64) -> f64) -> Result<Variable, String> {
    let right: f64 = rt.pop()?;
    let left: f64 = rt.pop()?;
    Ok(Variable::f64(function(left, right)))
}

fn native_abs(rt: &mut Runtime) -> Result<Variable, String> {
    unary(rt, math::abs)
}

fn native_sign(rt: &mut Runtime) -> Result<Variable, String> {
    unary(rt, math::sign)
}

fn native_int(rt: &mut Runtime) -> Result<Variable, String> {
    unary(rt, math::int)
}

fn native_round(rt: &mut Runtime) -> Result<Variable, String> {
    unary(rt, math::round)
}

fn native_floor(rt: &mut Runtime) -> Result<Variable, String> {
    unary(rt, math::floor)
}

fn native_ceil(rt: &mut Runtime) -> Result<Variable, String> {
    unary(rt, math::ceil)
}

fn native_pow(rt: &mut Runtime) -> Result<Variable, String> {
    binary(rt, math::pow)
}

fn native_sqrt(rt: &mut Runtime) -> Result<Variable, String> {
    checked_unary(rt, math::sqrt)
}

fn native_sin(rt: &mut Runtime) -> Result<Variable, String> {
    unary(rt, math::sin)
}

fn native_cos(rt: &mut Runtime) -> Result<Variable, String> {
    unary(rt, math::cos)
}

fn native_tan(rt: &mut Runtime) -> Result<Variable, String> {
    unary(rt, math::tan)
}

fn native_asin(rt: &mut Runtime) -> Result<Variable, String> {
    checked_unary(rt, math::asin)
}

fn native_acos(rt: &mut Runtime) -> Result<Variable, String> {
    checked_unary(rt, math::acos)
}

fn native_atan(rt: &mut Runtime) -> Result<Variable, String> {
    unary(rt, math::atan)
}

fn native_atan2(rt: &mut Runtime) -> Result<Variable, String> {
    binary(rt, math::atan2)
}

fn native_ln(rt: &mut Runtime) -> Result<Variable, String> {
    checked_unary(rt, math::ln)
}

fn native_log10(rt: &mut Runtime) -> Result<Variable, String> {
    checked_unary(rt, math::log10)
}

fn native_log2(rt: &mut Runtime) -> Result<Variable, String> {
    checked_unary(rt, math::log2)
}

fn native_exp(rt: &mut Runtime) -> Result<Variable, String> {
    unary(rt, math::exp)
}

fn native_min(rt: &mut Runtime) -> Result<Variable, String> {
    binary(rt, math::min)
}

fn native_max(rt: &mut Runtime) -> Result<Variable, String> {
    binary(rt, math::max)
}

fn native_random(_rt: &mut Runtime) -> Result<Variable, String> {
    Ok(Variable::f64(math::random()))
}

fn native_random_seed(rt: &mut Runtime) -> Result<(), String> {
    let seed: f64 = rt.pop()?;
    math::random_seed(seed);
    Ok(())
}

/// Registers the math commands into `module`.
///
/// Must run before source is loaded so Dyon's lifetime checker sees the
/// signatures.
pub(crate) fn register(module: &mut Module) {
    let mut unary_fn = |name: &str, f: fn(&mut Runtime) -> Result<Variable, String>| {
        module.add_str(name, f, Dfn::nl(vec![Type::F64], Type::F64));
    };
    unary_fn("abs", native_abs);
    unary_fn("sign", native_sign);
    unary_fn("int", native_int);
    unary_fn("round", native_round);
    unary_fn("floor", native_floor);
    unary_fn("ceil", native_ceil);
    unary_fn("sqrt", native_sqrt);
    unary_fn("sin", native_sin);
    unary_fn("cos", native_cos);
    unary_fn("tan", native_tan);
    unary_fn("asin", native_asin);
    unary_fn("acos", native_acos);
    unary_fn("atan", native_atan);
    unary_fn("ln", native_ln);
    unary_fn("log10", native_log10);
    unary_fn("log2", native_log2);
    unary_fn("exp", native_exp);

    let mut binary_fn = |name: &str, f: fn(&mut Runtime) -> Result<Variable, String>| {
        module.add_str(name, f, Dfn::nl(vec![Type::F64, Type::F64], Type::F64));
    };
    binary_fn("pow", native_pow);
    binary_fn("atan2", native_atan2);
    binary_fn("min", native_min);
    binary_fn("max", native_max);

    module.add_str("random", native_random, Dfn::nl(vec![], Type::F64));
    module.add_str(
        "random_seed",
        native_random_seed,
        Dfn::nl(vec![Type::F64], Type::Void),
    );
}
