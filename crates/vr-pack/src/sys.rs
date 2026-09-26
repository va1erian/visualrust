//! Raw Win32 boundary for `vr-pack`. Every `unsafe` call lives in this module
//! tree; callers see only [`crate::resources`] types.

mod resources;

pub(crate) use resources::patch_file;
