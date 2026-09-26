//! Resource patching read-back: patch a *copy* of the built `vr-runtime.exe`
//! with an icon, version info and manifest, then load that copy as a data file
//! and read the resources back with Win32.
//!
//! When no stub can be located the tests print `SKIP` and pass, matching the
//! export e2e. The real stub is never modified: every patch runs on a copy in a
//! temp directory.

mod common;

use std::path::{Path, PathBuf};

use common::{TempDir, runtime_stub};
use vr_pack::{PackError, Resources, VersionInfo, patch_resources};

use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows::Win32::System::LibraryLoader::{
    FindResourceExW, LOAD_LIBRARY_AS_DATAFILE, LoadLibraryExW, LoadResource, LockResource,
    SizeofResource,
};
use windows::core::PCWSTR;

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
const RT_MANIFEST: u16 = 24;
const LANG_EN_US: u16 = 0x0409;
const LANG_NEUTRAL: u16 = 0;
const PRODUCT_NAME: &str = "VisualRust Resource Demo";
const VERSION: &str = "1.2.3.4";
const MANIFEST_XML: &[u8] = b"<assembly><demo/></assembly>";

/// Copies the stub into `dir` under `name`, or returns `None` (caller prints
/// SKIP) with a copy failure reported as a real failure.
fn copy_stub(dir: &TempDir, name: &str) -> Option<PathBuf> {
    let stub = runtime_stub()?;
    let target = dir.join(name);
    std::fs::copy(&stub, &target).expect("copy stub into temp dir");
    Some(target)
}

fn skip(test: &str) {
    eprintln!("SKIP {test}: no vr-runtime.exe found (build vr-runtime or set VR_RUNTIME_STUB)");
}

#[test]
fn version_resource_is_patched_and_parses() {
    let Some(dir) = TempDir::create("version").ok() else {
        eprintln!("SKIP version read-back: no scratch dir");
        return;
    };
    let Some(exe) = copy_stub(&dir, "version.exe") else {
        skip("version read-back");
        return;
    };

    let resources = Resources::new().with_version(VersionInfo::new(PRODUCT_NAME, "demo", VERSION));
    patch_resources(&exe, &resources).expect("patch version");

    let module = load_datafile(&exe).expect("load patched exe");
    let raw = find_resource(&module, RT_VERSION, 1, LANG_EN_US).expect("RT_VERSION present");
    let text = utf16_lossy(&raw);
    assert!(text.contains("VS_VERSION_INFO"), "root key present");
    assert!(
        text.contains(PRODUCT_NAME),
        "product name present in {text:?}"
    );
    assert!(text.contains(VERSION), "version string present");

    // Windows' own parser must accept the block: query the fixed file info.
    let fixed = query_fixed_info(&exe).expect("VerQueryValueW parses VS_FIXEDFILEINFO");
    assert_eq!(
        u32::from_le_bytes(fixed[0..4].try_into().expect("dword")),
        0xFEEF_04BD
    );
    let file_version = u32::from_le_bytes(fixed[8..12].try_into().expect("dword"));
    assert_eq!(file_version, (1 << 16) | 2);
    let build = u32::from_le_bytes(fixed[12..16].try_into().expect("dword"));
    assert_eq!(build, (3 << 16) | 4);

    eprintln!(
        "version read-back: RT_VERSION present ({} bytes), VerQueryValueW returned \
         FILEVERSION {}.{}.{}.{}",
        raw.len(),
        file_version >> 16,
        file_version & 0xFFFF,
        build >> 16,
        build & 0xFFFF
    );
}

#[test]
fn icon_group_and_image_are_patched() {
    let Some(dir) = TempDir::create("icon").ok() else {
        eprintln!("SKIP icon read-back: no scratch dir");
        return;
    };
    let Some(exe) = copy_stub(&dir, "icon.exe") else {
        skip("icon read-back");
        return;
    };
    let ico = common::sample_ico();

    let resources = Resources::new().with_icon(ico.clone());
    patch_resources(&exe, &resources).expect("patch icon");

    let module = load_datafile(&exe).expect("load patched exe");
    let group = find_resource(&module, RT_GROUP_ICON, 1, LANG_NEUTRAL).expect("RT_GROUP_ICON");
    assert_eq!(
        u16::from_le_bytes([group[4], group[5]]),
        1,
        "one image entry"
    );
    let image_id = u16::from_le_bytes([group[18], group[19]]);
    assert_eq!(image_id, 1, "group entry points at RT_ICON id 1");

    let image = find_resource(&module, RT_ICON, image_id, LANG_NEUTRAL).expect("RT_ICON");
    assert_eq!(image, ico[22..], "RT_ICON matches the .ico image payload");

    eprintln!(
        "icon read-back: RT_GROUP_ICON ({} bytes) -> RT_ICON id {} ({} bytes)",
        group.len(),
        image_id,
        image.len()
    );
}

#[test]
fn manifest_resource_is_patched() {
    let Some(dir) = TempDir::create("manifest").ok() else {
        eprintln!("SKIP manifest read-back: no scratch dir");
        return;
    };
    let Some(exe) = copy_stub(&dir, "manifest.exe") else {
        skip("manifest read-back");
        return;
    };

    let resources = Resources::new().with_manifest(MANIFEST_XML.to_vec());
    patch_resources(&exe, &resources).expect("patch manifest");

    let module = load_datafile(&exe).expect("load patched exe");
    let built = find_resource(&module, RT_MANIFEST, 1, LANG_EN_US).expect("RT_MANIFEST");
    assert_eq!(built, MANIFEST_XML, "manifest bytes round-tripped");

    eprintln!(
        "manifest read-back: RT_MANIFEST present ({} bytes)",
        built.len()
    );
}

#[test]
fn a_failed_patch_leaves_the_input_and_no_staging_file() {
    let dir = TempDir::create("untouched").expect("temp dir");
    let exe = dir.join("not-a-pe.exe");
    let original = b"this is definitely not a PE image".to_vec();
    std::fs::write(&exe, &original).expect("write fake exe");

    let resources = Resources::new().with_version(VersionInfo::new("p", "p", "1.0.0"));
    let err = patch_resources(&exe, &resources).expect_err("non-PE must fail");

    assert!(
        matches!(err, PackError::ResourcePatch { .. }),
        "got {err:?}"
    );
    assert_eq!(
        std::fs::read(&exe).expect("read input"),
        original,
        "input must be byte-identical after a failed patch"
    );

    let staging = std::fs::read_dir(dir.path())
        .expect("read temp dir")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains("vrpatch"))
        .count();
    assert_eq!(staging, 0, "the staging copy must be cleaned up");

    eprintln!("failure path: non-PE rejected, input untouched, staging removed");
}

/// An owned module handle loaded with `LOAD_LIBRARY_AS_DATAFILE`; freed on drop.
struct Module(HMODULE);

impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: the handle came from LoadLibraryExW and is freed exactly once.
        unsafe {
            let _ = FreeLibrary(self.0);
        }
    }
}

fn load_datafile(path: &Path) -> Result<Module, String> {
    let wide = wide_path(path);
    // SAFETY: `wide` is NUL-terminated and outlives the call; no file handle is
    // supplied, which is allowed for `LOAD_LIBRARY_AS_DATAFILE`.
    let handle = unsafe { LoadLibraryExW(PCWSTR(wide.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE) }
        .map_err(|error| format!("LoadLibraryExW failed: {error}"))?;
    Ok(Module(handle))
}

/// Reads `type_id`/`name_id` at `language`; `None` when the resource is absent.
fn find_resource(module: &Module, type_id: u16, name_id: u16, language: u16) -> Option<Vec<u8>> {
    let type_ptr = PCWSTR(type_id as usize as *const u16);
    let name_ptr = PCWSTR(name_id as usize as *const u16);
    // SAFETY: both pointers are MAKEINTRESOURCE-style integers, interpreted by
    // Windows as ids because they fit in 16 bits.
    unsafe {
        let resource = FindResourceExW(Some(module.0), type_ptr, name_ptr, language);
        if resource.is_invalid() {
            return None;
        }
        let size = SizeofResource(Some(module.0), resource);
        if size == 0 {
            return None;
        }
        let loaded = LoadResource(Some(module.0), resource).ok()?;
        let data = LockResource(loaded).cast::<u8>();
        if data.is_null() {
            return None;
        }
        // SAFETY: the resource data is valid for `size` bytes until the module
        // is freed, and the copy is made immediately.
        Some(std::slice::from_raw_parts(data, size as usize).to_vec())
    }
}

/// Asks `version.dll` for the `VS_FIXEDFILEINFO` of `path`, proving Windows can
/// parse the block this crate wrote.
fn query_fixed_info(path: &Path) -> Option<Vec<u8>> {
    let wide = wide_path(path);
    // SAFETY: `wide` is NUL-terminated and outlives both calls below.
    let size = unsafe { GetFileVersionInfoSizeW(PCWSTR(wide.as_ptr()), None) };
    if size == 0 {
        return None;
    }
    let mut data = vec![0u8; size as usize];
    // SAFETY: `data` is exactly `size` bytes and the length matches.
    unsafe {
        GetFileVersionInfoW(PCWSTR(wide.as_ptr()), None, size, data.as_mut_ptr().cast()).ok()?;
    }

    let subblock = wide_str("\\");
    let mut buffer: *mut core::ffi::c_void = std::ptr::null_mut();
    let mut length: u32 = 0;
    // SAFETY: `data` holds the version block returned above; `subblock` is a
    // NUL-terminated string; the out pointers are valid for the call.
    let ok = unsafe {
        VerQueryValueW(
            data.as_ptr().cast(),
            PCWSTR(subblock.as_ptr()),
            &mut buffer,
            &mut length,
        )
    };
    if !ok.as_bool() || buffer.is_null() || length < 16 {
        return None;
    }
    // SAFETY: VerQueryValueW pointed `buffer` at `length` valid bytes inside
    // `data`, which is still alive.
    Some(unsafe { std::slice::from_raw_parts(buffer.cast::<u8>(), length as usize).to_vec() })
}

fn utf16_lossy(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    String::from_utf16_lossy(&units)
}

fn wide_str(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
