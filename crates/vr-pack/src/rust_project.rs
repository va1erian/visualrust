//! Optional "build as a Rust project" export mode (#70).
//!
//! The default export copies a prebuilt `vr-runtime.exe` stub and appends the
//! bundle (#52), so end users need no Rust toolchain but cannot link native Rust
//! extensions. This mode instead emits a Cargo project that depends on
//! `vr-runtime`, embeds the same bundle with `include_bytes!`, and calls the
//! runtime loader from `main`. The user adds `[dependencies]` for their own
//! extension crates and `cargo build` produces one self-contained executable.
//!
//! Trade-off: this path needs a toolchain on the build machine and is slower and
//! larger than copying the stub, but it is the only one that links extra native
//! crates. The stub model stays the default and is untouched.
//!
//! `cargo build` is opt-in through [`RustProjectOptions::with_build`] because it
//! requires a toolchain and can take minutes; [`emit_rust_project`] always writes
//! the project regardless.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use vr_core::manifest::Manifest;
use vr_core::project::MANIFEST_FILE;

use crate::bundle::Bundle;
use crate::error::PackError;

/// Where the generated project finds `vr-runtime` at build time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeDependency {
    /// A path dependency, normally the IDE's bundled `runtime/vr-runtime`.
    Path(PathBuf),
    /// A git dependency; pin a `rev` so an export rebuilds reproducibly.
    Git { url: String, rev: Option<String> },
}

/// How [`emit_rust_project`] should emit, and whether to then compile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustProjectOptions {
    dependency: RuntimeDependency,
    build: bool,
}

impl RustProjectOptions {
    pub fn new(dependency: RuntimeDependency) -> Self {
        Self {
            dependency,
            build: false,
        }
    }

    /// Enables the opt-in `cargo build` step.
    #[must_use]
    pub fn with_build(mut self, build: bool) -> Self {
        self.build = build;
        self
    }

    pub fn dependency(&self) -> &RuntimeDependency {
        &self.dependency
    }

    pub fn build_requested(&self) -> bool {
        self.build
    }
}

/// A project emitted by [`emit_rust_project`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustProject {
    root: PathBuf,
    name: String,
    bundle_len: usize,
    built: Option<BuildOutcome>,
}

impl RustProject {
    /// The directory the project was written to.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The crate/package name taken from the project manifest.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Serialized bundle size embedded as `app.vrbundle`.
    pub fn bundle_len(&self) -> usize {
        self.bundle_len
    }

    /// The build result when one was requested, otherwise `None`.
    pub fn build(&self) -> Option<&BuildOutcome> {
        self.built.as_ref()
    }
}

/// A successful `cargo build` of an emitted project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutcome {
    /// The project root that was built.
    pub root: PathBuf,
    /// Cargo's captured stdout, useful for showing the build log.
    pub stdout: String,
}

/// Writes a Cargo project for `project_dir` into `out_dir`.
///
/// The manifest is loaded from the project so the emitted crate carries the same
/// name and version, and every referenced file is read into the embedded bundle.
/// When `options.build_requested()` is set, [`build_rust_project`] runs after the
/// files are written; a missing toolchain is then [`PackError::CargoUnavailable`].
pub fn emit_rust_project(
    project_dir: impl AsRef<Path>,
    out_dir: impl AsRef<Path>,
    options: &RustProjectOptions,
) -> Result<RustProject, PackError> {
    let project_dir = project_dir.as_ref();
    let out_dir = out_dir.as_ref();
    let manifest = Manifest::load(project_dir.join(MANIFEST_FILE))?;
    let bundle = Bundle::from_project(project_dir, manifest.clone())?;
    let bytes = bundle.to_bytes()?;

    let cargo_toml = out_dir.join("Cargo.toml");
    if cargo_toml.exists() {
        return Err(PackError::RustProjectExists { path: cargo_toml });
    }

    write_file(
        &cargo_toml,
        manifest_cargo_toml(&manifest, options.dependency()).as_bytes(),
    )?;
    write_file(&out_dir.join("src").join("main.rs"), MAIN_RS.as_bytes())?;
    write_file(&out_dir.join("app.vrbundle"), &bytes)?;
    write_file(&out_dir.join("README.md"), README.as_bytes())?;

    let built = if options.build {
        Some(build_rust_project(out_dir)?)
    } else {
        None
    };

    Ok(RustProject {
        root: out_dir.to_path_buf(),
        name: manifest.project.name.clone(),
        bundle_len: bytes.len(),
        built,
    })
}

/// Runs `cargo build` in an emitted project root.
///
/// Returns [`PackError::CargoUnavailable`] before spawning anything when cargo is
/// absent, so callers can skip the step instead of treating it as a build error.
pub fn build_rust_project(root: impl AsRef<Path>) -> Result<BuildOutcome, PackError> {
    let root = root.as_ref();
    if !cargo_available() {
        return Err(PackError::CargoUnavailable);
    }

    // A nested build must not inherit the caller's target dir: the outer cargo
    // holds that lock, so sharing it would deadlock the test that invoked us.
    let target = root.join("target");
    let output = Command::new("cargo")
        .arg("build")
        .current_dir(root)
        .env("CARGO_TARGET_DIR", &target)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|source| PackError::CargoSpawn {
            root: root.to_path_buf(),
            source,
        })?;

    if !output.status.success() {
        return Err(PackError::CargoBuildFailed {
            root: root.to_path_buf(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    Ok(BuildOutcome {
        root: root.to_path_buf(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
    })
}

/// Whether a `cargo` executable can be run; the gate for building.
pub fn cargo_available() -> bool {
    Command::new("cargo")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), PackError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| PackError::RustProjectWrite {
            path: path.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(path, bytes).map_err(|source| PackError::RustProjectWrite {
        path: path.to_path_buf(),
        source,
    })
}

fn manifest_cargo_toml(manifest: &Manifest, dependency: &RuntimeDependency) -> String {
    // The empty `[workspace]` makes the emitted crate its own workspace root, so
    // it still builds when written below another Cargo project's directory.
    format!(
        "[package]\n\
         name = \"{name}\"\n\
         version = \"{version}\"\n\
         edition = \"2024\"\n\n\
         [dependencies]\n\
         vr-runtime = {dependency}\n\n\
         [workspace]\n",
        name = escape(&manifest.project.name),
        version = escape(&manifest.project.version),
        dependency = render_dependency(dependency),
    )
}

fn render_dependency(dependency: &RuntimeDependency) -> String {
    match dependency {
        RuntimeDependency::Path(path) => {
            format!("{{ path = \"{}\" }}", escape(&path.to_string_lossy()))
        }
        RuntimeDependency::Git { url, rev } => match rev {
            Some(rev) => format!("{{ git = \"{}\", rev = \"{}\" }}", escape(url), escape(rev)),
            None => format!("{{ git = \"{}\" }}", escape(url)),
        },
    }
}

/// Escapes a value for a TOML basic string. Windows paths carry backslashes,
/// which TOML would otherwise read as escape sequences.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(c),
        }
    }
    out
}

/// `src/main.rs` of the generated project, kept verbatim so the emitted code is
/// reviewable and independent of emit-time string formatting.
const MAIN_RS: &str = r#"//! Generated by VisualRust's "build as a Rust project" export (#70).
//!
//! The app bundle is embedded with `include_bytes!`, so the compiled binary
//! carries the whole app. The runtime's loader takes a file path, so the bytes
//! are staged to a temporary file for the run and removed afterwards.

use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(report) => {
            println!(
                "vr-app: ran `{}` ({:?}, {} entries)",
                report.entry.display(),
                report.kind,
                report.entries.len()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("vr-app: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<vr_runtime::RunReport, Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("vr-app-{}.vrbundle", std::process::id()));
    std::fs::write(&path, include_bytes!("../app.vrbundle"))?;
    let report = vr_runtime::run_embedded_from(&path);
    let _ = std::fs::remove_file(&path);
    Ok(report?)
}
"#;

const README: &str = r#"# Generated VisualRust project

This crate was emitted by VisualRust's optional **build as a Rust project**
export (#70). It embeds the app bundle (`app.vrbundle`) and runs it through
`vr-runtime` from `src/main.rs`.

## Trade-off vs the stub export

- **Stub export (default):** copies a prebuilt `vr-runtime.exe` and appends the
  bundle. End users need no Rust toolchain, exports are fast, and native code is
  limited to the DLLs listed in the manifest.
- **This mode:** needs a Rust toolchain for `cargo build`. Exports are slower and
  the binary is larger, but you can add `[dependencies]` on your own native
  crates and link them directly.

Add your extension crates to `[dependencies]`, then run `cargo build`.
"#;
