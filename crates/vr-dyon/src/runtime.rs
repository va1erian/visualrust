//! Safe wrapper around a loaded Dyon module and its runtime.

use std::path::Path;
use std::sync::Arc;

use dyon::embed::PopVariable;
use dyon::{Module, Runtime, Variable};

use crate::error::DyonError;
use crate::native;

/// A Dyon program that has been compiled and is ready to run.
pub struct DyonRuntime {
    runtime: Runtime,
    module: Arc<Module>,
}

impl DyonRuntime {
    /// Compiles `source`, registering only the built-in native functions.
    ///
    /// `source_name` is only used to label diagnostics. For a program that
    /// needs extra native modules, use [`from_source_with`](Self::from_source_with).
    pub fn from_source(source_name: &str, source: &str) -> Result<Self, DyonError> {
        Self::from_source_with(source_name, source, |_| {})
    }

    /// Compiles `source`, letting `register` add extra native modules first.
    ///
    /// The built-in natives are registered before `register` runs, and both
    /// run before the source is loaded so Dyon's lifetime checker sees every
    /// signature. This is the extension point that keeps optional modules out
    /// of `vr-dyon`'s dependency graph: the crate that owns the natives passes
    /// its own `register` function here.
    pub fn from_source_with(
        source_name: &str,
        source: &str,
        register: impl FnOnce(&mut Module),
    ) -> Result<Self, DyonError> {
        let mut module = Module::new();
        native::register(&mut module);
        register(&mut module);
        dyon::load_str(source_name, Arc::new(source.to_owned()), &mut module)
            .map_err(|message| DyonError::compile(source_name, message))?;
        // Dyon does not expose a loaded function's signature, so the handler
        // arities the UI dispatch needs are scanned from the source once.
        #[cfg(feature = "ui")]
        crate::ui::set_arities(source);
        Ok(Self {
            runtime: Runtime::new(),
            module: Arc::new(module),
        })
    }

    /// Compiles a program read from `path`.
    ///
    /// The file is read here rather than via `dyon::load` so IO failures stay
    /// typed instead of being flattened into Dyon's error string.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, DyonError> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path).map_err(|source| DyonError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_source(&path.to_string_lossy(), &source)
    }

    /// Runs the program's `main` function, then hosts any UI it registered.
    ///
    /// `ui_run` only registers a window; once `main` has returned this takes
    /// that plan and starts the xui message loop with the Dyon runtime moved
    /// into the app. A program that asks for a window on a session that cannot
    /// create one fails with [`DyonError::NoWindow`] instead of a generic
    /// runtime error, so a UI test can skip.
    pub fn run(&mut self) -> Result<(), DyonError> {
        #[cfg(feature = "ui")]
        crate::ui::reset_status();

        let result = self.runtime.run(&self.module).map_err(DyonError::runtime);
        if result.is_err() {
            // A failed `main` owns the error; discard any half-built plan.
            #[cfg(feature = "ui")]
            let _ = crate::ui::take_pending();
            return result;
        }

        #[cfg(feature = "ui")]
        {
            if let Some(pending) = crate::ui::take_pending() {
                // Move the runtime and module into the app so a handler runs on
                // a program that is no longer on this stack (no re-entrancy).
                let runtime = std::mem::replace(&mut self.runtime, Runtime::new());
                let module = Arc::clone(&self.module);
                match crate::ui::run_host(pending, runtime, module) {
                    Ok(()) => crate::ui::set_status(crate::ui::UiStatus::Ran),
                    Err(crate::ui::HostError::NoWindow) => {
                        crate::ui::set_status(crate::ui::UiStatus::NoWindow);
                        return Err(DyonError::NoWindow);
                    }
                    Err(crate::ui::HostError::Failed(message)) => {
                        return Err(DyonError::runtime(message));
                    }
                }
            }
            let _ = crate::ui::take_status();
        }

        Ok(())
    }

    /// Calls a function by name without a return value.
    pub fn call(&mut self, function: &str, args: &[Variable]) -> Result<(), DyonError> {
        self.runtime
            .call_str(function, args, &self.module)
            .map_err(DyonError::runtime)
    }

    /// Calls a function by name and converts its result to `T`.
    pub fn call_ret<T: PopVariable>(
        &mut self,
        function: &str,
        args: &[Variable],
    ) -> Result<T, DyonError> {
        let value = self
            .runtime
            .call_str_ret(function, args, &self.module)
            .map_err(DyonError::runtime)?;
        T::pop_var(&self.runtime, self.runtime.get(&value)).map_err(DyonError::runtime)
    }

    /// The compiled module, for callers that need Dyon's own API.
    pub fn module(&self) -> &Module {
        &self.module
    }
}
