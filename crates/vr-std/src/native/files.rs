//! Dyon native functions for the file and directory commands.
//!
//! A handle crosses as a Dyon custom object (`Arc<Mutex<FileHandle>>`), exactly
//! as the database and UI bindings do, so `open_file` returns one value and
//! every later command takes it by reference. Any [`FileError`] is rendered
//! once into the only error a Dyon native may return, a `String`.
//!
//! Dyon has no byte-string type, so `read_data` and `write_data` use arrays of
//! whole numbers in `0..=255`. `examine_directory` returns an array of objects
//! with a `name` string and a numeric `type` flag (`0` file, `1` directory,
//! `2` other).

use std::collections::HashMap;
use std::sync::Arc;

use dyon::embed::to_rust_object;
use dyon::{Dfn, Module, Runtime, RustObject, Type, Variable};

use crate::error::FileError;
use crate::files::{self, DirEntry, FileHandle};

/// Registers the file commands into `module`.
///
/// Must run before source is loaded so Dyon's lifetime checker sees the
/// signatures.
pub(crate) fn register(module: &mut Module) {
    let file = ad_hoc("File");
    module.add_str(
        "open_file",
        native_open_file,
        Dfn::nl(vec![Type::Str, Type::Str], file.clone()),
    );
    module.add_str(
        "read_string",
        native_read_string,
        Dfn::nl(vec![file.clone()], Type::Str),
    );
    module.add_str(
        "write_string",
        native_write_string,
        Dfn::nl(vec![file.clone(), Type::Str], Type::Void),
    );
    module.add_str(
        "read_data",
        native_read_data,
        Dfn::nl(vec![file.clone()], Type::Array(Box::new(Type::F64))),
    );
    module.add_str(
        "write_data",
        native_write_data,
        Dfn::nl(
            vec![file.clone(), Type::Array(Box::new(Type::F64))],
            Type::Void,
        ),
    );
    module.add_str(
        "close_file",
        native_close_file,
        Dfn::nl(vec![file.clone()], Type::Void),
    );
    module.add_str("eof", native_eof, Dfn::nl(vec![file.clone()], Type::Bool));
    module.add_str(
        "file_size",
        native_file_size,
        Dfn::nl(vec![file], Type::F64),
    );
    module.add_str(
        "delete_file",
        native_delete_file,
        Dfn::nl(vec![Type::Str], Type::Void),
    );
    module.add_str(
        "copy_file",
        native_copy_file,
        Dfn::nl(vec![Type::Str, Type::Str], Type::F64),
    );
    module.add_str(
        "rename_file",
        native_rename_file,
        Dfn::nl(vec![Type::Str, Type::Str], Type::Void),
    );
    module.add_str(
        "create_directory",
        native_create_directory,
        Dfn::nl(vec![Type::Str], Type::Void),
    );
    module.add_str(
        "delete_directory",
        native_delete_directory,
        Dfn::nl(vec![Type::Str], Type::Void),
    );
    module.add_str(
        "examine_directory",
        native_examine_directory,
        Dfn::nl(vec![Type::Str], Type::Array(Box::new(Type::Object))),
    );
    module.add_str(
        "get_path_part",
        native_get_path_part,
        Dfn::nl(vec![Type::Str, Type::Str], Type::Str),
    );
}

/// An ad-hoc type named `name`, so a handle from `open_file` is accepted only
/// by the file commands.
fn ad_hoc(name: &str) -> Type {
    Type::AdHoc(Arc::new(name.to_owned()), Box::new(Type::Any))
}

fn native_open_file(rt: &mut Runtime) -> Result<Variable, String> {
    let mode: String = rt.pop()?;
    let path: String = rt.pop()?;
    let handle = files::open_file(&path, &mode).map_err(render)?;
    Ok(Variable::RustObject(to_rust_object(handle)))
}

fn native_read_string(rt: &mut Runtime) -> Result<Variable, String> {
    let object: RustObject = rt.pop()?;
    let text = with_file(&object, files::read_string)?;
    Ok(Variable::Str(Arc::new(text)))
}

fn native_write_string(rt: &mut Runtime) -> Result<(), String> {
    let text: String = rt.pop()?;
    let object: RustObject = rt.pop()?;
    with_file(&object, |handle| files::write_string(handle, &text))
}

fn native_read_data(rt: &mut Runtime) -> Result<Variable, String> {
    let object: RustObject = rt.pop()?;
    let bytes = with_file(&object, files::read_data)?;
    Ok(bytes_variable(bytes))
}

fn native_write_data(rt: &mut Runtime) -> Result<(), String> {
    let raw: Variable = rt.pop()?;
    let object: RustObject = rt.pop()?;
    let bytes = read_bytes(rt, &raw)?;
    with_file(&object, |handle| files::write_data(handle, &bytes))
}

fn native_close_file(rt: &mut Runtime) -> Result<(), String> {
    let object: RustObject = rt.pop()?;
    with_file(&object, |handle| {
        files::close_file(handle);
        Ok(())
    })
}

fn native_eof(rt: &mut Runtime) -> Result<Variable, String> {
    let object: RustObject = rt.pop()?;
    let at_end = with_file(&object, files::eof)?;
    Ok(Variable::bool(at_end))
}

fn native_file_size(rt: &mut Runtime) -> Result<Variable, String> {
    let object: RustObject = rt.pop()?;
    let size = with_file(&object, files::file_size)?;
    Ok(Variable::f64(size as f64))
}

fn native_delete_file(rt: &mut Runtime) -> Result<(), String> {
    let path: String = rt.pop()?;
    files::delete_file(&path).map_err(render)
}

fn native_copy_file(rt: &mut Runtime) -> Result<Variable, String> {
    let to: String = rt.pop()?;
    let from: String = rt.pop()?;
    let copied = files::copy_file(&from, &to).map_err(render)?;
    Ok(Variable::f64(copied as f64))
}

fn native_rename_file(rt: &mut Runtime) -> Result<(), String> {
    let to: String = rt.pop()?;
    let from: String = rt.pop()?;
    files::rename_file(&from, &to).map_err(render)
}

fn native_create_directory(rt: &mut Runtime) -> Result<(), String> {
    let path: String = rt.pop()?;
    files::create_directory(&path).map_err(render)
}

fn native_delete_directory(rt: &mut Runtime) -> Result<(), String> {
    let path: String = rt.pop()?;
    files::delete_directory(&path).map_err(render)
}

fn native_examine_directory(rt: &mut Runtime) -> Result<Variable, String> {
    let path: String = rt.pop()?;
    let entries = files::examine_directory(&path).map_err(render)?;
    Ok(entries_variable(entries))
}

fn native_get_path_part(rt: &mut Runtime) -> Result<Variable, String> {
    let part: String = rt.pop()?;
    let path: String = rt.pop()?;
    let value = files::get_path_part(&path, &part).map_err(render)?;
    Ok(Variable::Str(Arc::new(value)))
}

/// Renders a typed failure as the `String` Dyon expects.
fn render(error: FileError) -> String {
    error.to_string()
}

/// Runs `action` with exclusive access to the handle, mapping lock and downcast
/// failures to Dyon errors.
fn with_file<T>(
    object: &RustObject,
    action: impl FnOnce(&mut FileHandle) -> Result<T, FileError>,
) -> Result<T, String> {
    let mut guard = object.lock().map_err(|_| FileError::Poisoned.to_string())?;
    let handle = guard
        .downcast_mut::<FileHandle>()
        .ok_or_else(|| FileError::NotAHandle.to_string())?;
    action(handle).map_err(render)
}

/// Reads a Dyon byte array, rejecting anything that is not a whole number in
/// `0..=255` so a typo cannot silently truncate binary data.
fn read_bytes(rt: &Runtime, variable: &Variable) -> Result<Vec<u8>, String> {
    let resolved = rt.get(variable);
    let Variable::Array(items) = resolved else {
        return Err(FileError::NotAnArray(resolved.typeof_var().to_string()).to_string());
    };
    let mut bytes = Vec::with_capacity(items.len());
    for item in items.iter() {
        match rt.get(item) {
            Variable::F64(number, _) if is_byte(*number) => bytes.push(*number as u8),
            other => {
                return Err(
                    FileError::InvalidByteElement(other.typeof_var().to_string()).to_string(),
                );
            }
        }
    }
    Ok(bytes)
}

/// Wraps bytes as Dyon's array type. `Variable` is not `Sync`, so clippy's
/// non-`Send`/`Sync` warning on the `Arc` is unavoidable when matching Dyon.
#[allow(clippy::arc_with_non_send_sync)]
fn bytes_variable(bytes: Vec<u8>) -> Variable {
    Variable::Array(Arc::new(
        bytes
            .into_iter()
            .map(|byte| Variable::f64(f64::from(byte)))
            .collect(),
    ))
}

/// Wraps directory entries as an array of `{ name, type }` objects.
#[allow(clippy::arc_with_non_send_sync)]
fn entries_variable(entries: Vec<DirEntry>) -> Variable {
    let items: Vec<Variable> = entries
        .into_iter()
        .map(|entry| {
            let mut fields: HashMap<Arc<String>, Variable> = HashMap::new();
            fields.insert(
                Arc::new("name".to_owned()),
                Variable::Str(Arc::new(entry.name)),
            );
            fields.insert(
                Arc::new("type".to_owned()),
                Variable::f64(entry.kind.flag()),
            );
            Variable::Object(Arc::new(fields))
        })
        .collect();
    Variable::Array(Arc::new(items))
}

/// Whether a Dyon number is a byte value (`0..=255`, no fraction).
fn is_byte(number: f64) -> bool {
    number.is_finite() && number.fract() == 0.0 && (0.0..=255.0).contains(&number)
}
