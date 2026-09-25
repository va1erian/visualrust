//! Builds the vendored Scintilla C++ sources into a static library with `cc`.
//!
//! Everything is checked into `vendor/scintilla`, so the build performs no
//! network or git access. The source list mirrors the upstream `win32/makefile`
//! `COMPONENT_OBJS` rule; the three `win32` files the issue's list omits
//! (`ListBox`, `SurfaceGDI`, `SurfaceD2D`) are required by `PlatWin` and by
//! `Surface::Allocate` / `Font::Allocate`, which live in `SurfaceD2D.cxx`.

use std::path::Path;

/// The portable `src/*.cxx` translation units Scintilla requires.
const SRC_SOURCES: &[&str] = &[
    "AutoComplete",
    "CallTip",
    "CaseConvert",
    "CaseFolder",
    "CellBuffer",
    "ChangeHistory",
    "CharacterCategoryMap",
    "CharacterType",
    "CharClassify",
    "ContractionState",
    "DBCS",
    "Decoration",
    "Document",
    "EditModel",
    "Editor",
    "EditView",
    "Geometry",
    "Indicator",
    "KeyMap",
    "LineMarker",
    "MarginView",
    "PerLine",
    "PositionCache",
    "RESearch",
    "RunStyles",
    "ScintillaBase",
    "Selection",
    "Style",
    "UniConversion",
    "UniqueString",
    "UndoHistory",
    "ViewStyle",
    "XPM",
];

/// The `win32/*.cxx` translation units, including the platform backend.
const WIN32_SOURCES: &[&str] = &[
    "HanjaDic",
    "ListBox",
    "PlatWin",
    "SurfaceD2D",
    "SurfaceGDI",
    "ScintillaWin",
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=vendor/scintilla/src");
    println!("cargo:rerun-if-changed=vendor/scintilla/include");
    println!("cargo:rerun-if-changed=vendor/scintilla/win32");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    let root = Path::new(&manifest).join("vendor").join("scintilla");
    let src = root.join("src");
    let win32 = root.join("win32");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .define("STATIC_BUILD", None)
        .define("NDEBUG", None)
        .include(root.join("include"))
        .include(&src)
        .include(&win32)
        // Third-party code: its warnings are not ours to fix.
        .warnings(false)
        .flag_if_supported("/EHsc");

    for name in SRC_SOURCES {
        build.file(src.join(format!("{name}.cxx")));
    }
    for name in WIN32_SOURCES {
        build.file(win32.join(format!("{name}.cxx")));
    }

    build.compile("scintilla");

    for lib in [
        "user32", "gdi32", "imm32", "ole32", "oleaut32", "uuid", "advapi32",
    ] {
        println!("cargo:rustc-link-lib={lib}");
    }
}
