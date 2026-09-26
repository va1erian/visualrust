//! Win32 resource update calls: `BeginUpdateResourceW`, `UpdateResourceW` and
//! `EndUpdateResourceW`.
//!
//! This is the only file allowed to use `unsafe`; every block carries a
//! `// SAFETY:` note. The API is transactional at the file level: changes are
//! buffered by Windows and only written when `EndUpdateResourceW` runs, so
//! passing `discard = true` rolls the image back after a failed update.

#![allow(unsafe_code)]

use core::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::LibraryLoader::{
    BeginUpdateResourceW, EndUpdateResourceW, UpdateResourceW,
};
use windows::core::PCWSTR;

use crate::resources::PatchOp;

/// Applies `ops` to the PE image at `path`, atomically from the caller's view.
///
/// Returns a plain message on failure; the caller wraps it in a typed
/// [`crate::PackError`]. The file is left byte-identical if any step fails
/// because `EndUpdateResourceW` is told to discard the buffered changes.
pub(crate) fn patch_file(path: &Path, ops: &[PatchOp]) -> Result<(), String> {
    let wide = wide_path(path);
    // SAFETY: `wide` is NUL-terminated and outlives the call; `false` keeps the
    // stub's existing resources (the manifest, prior icon) instead of wiping
    // them, and the returned handle is owned until EndUpdateResourceW.
    let handle =
        unsafe { BeginUpdateResourceW(PCWSTR(wide.as_ptr()), false) }.map_err(begin_error)?;

    let mut failure = None;
    for op in ops {
        if let Err(message) = apply(handle, op) {
            failure = Some(message);
            break;
        }
    }

    let discard = failure.is_some();
    // SAFETY: `handle` came from BeginUpdateResourceW above and has not been
    // passed to EndUpdateResourceW yet; it is consumed exactly once here.
    let end = unsafe { EndUpdateResourceW(handle, discard) };

    match (failure, end) {
        (Some(message), _) => Err(message),
        (None, Err(error)) => Err(format!("EndUpdateResourceW failed: {error}")),
        (None, Ok(())) => Ok(()),
    }
}

fn apply(handle: HANDLE, op: &PatchOp) -> Result<(), String> {
    // Resource ids fit in 16 bits, so `MAKEINTRESOURCE` is just the widened id.
    let type_ptr = PCWSTR(op.type_id as usize as *const u16);
    let name_ptr = PCWSTR(op.name_id as usize as *const u16);

    let length = u32::try_from(op.data.len())
        .map_err(|_| format!("resource {} is too large", op.type_id))?;
    let data = op.data.as_ptr().cast::<c_void>();

    // SAFETY: the type, name and data pointers all reference buffers that stay
    // alive for the duration of the call; `length` is the data buffer's length,
    // so Windows copies exactly that many bytes.
    unsafe { UpdateResourceW(handle, type_ptr, name_ptr, op.language, Some(data), length) }
        .map_err(|error| format!("UpdateResourceW(type {}) failed: {error}", op.type_id))
}

fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn begin_error(error: windows::core::Error) -> String {
    format!("BeginUpdateResourceW failed: {error}")
}
