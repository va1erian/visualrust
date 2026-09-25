//! Packaged app runtime: hosts a compiled Dyon program in a win32ui window.
//!
//! [`RuntimeApp`] is the bridge between win32ui's retained message loop and a
//! Dyon module: each incoming [`HandlerId`] is mapped to the name of a Dyon
//! function, which the runtime calls once per message. Dispatch is kept free of
//! win32ui types so it can be exercised headlessly.

#![forbid(unsafe_code)]

mod app;

pub use app::{DispatchError, HandlerId, RuntimeApp};
