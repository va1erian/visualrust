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
    /// Compiles `source`, registering the built-in native functions first.
    ///
    /// `source_name` is only used to label diagnostics.
    pub fn from_source(source_name: &str, source: &str) -> Result<Self, DyonError> {
        let mut module = Module::new();
        native::register(&mut module);
        dyon::load_str(source_name, Arc::new(source.to_owned()), &mut module)
            .map_err(|message| DyonError::compile(source_name, message))?;
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

    /// Runs the program's `main` function.
    pub fn run(&mut self) -> Result<(), DyonError> {
        self.runtime.run(&self.module).map_err(DyonError::runtime)
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
