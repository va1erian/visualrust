//! The IDE's internal editor document.
//!
//! The workspace's `vr-editor` crate is still the bootstrap placeholder from
//! issue #1, so the IDE keeps a tiny document type of its own here: where the
//! text came from, the name to show, and the text as last loaded or saved.
//! Issue #29 will move the document model into `vr-editor` and generalise this.

use std::io;
use std::path::{Path, PathBuf};

use vr_core::manifest::Manifest;

/// The manifest filename every project directory carries.
const MANIFEST_FILE: &str = "vrproj.toml";

/// The sample opened when no path and no nearby project are available, relative
/// to the working directory. It matches the example the runtime smoke test uses.
pub const SAMPLE_RELATIVE: &str = "examples/hello-window/src/main.dyon";

/// The text shown when even the on-disk sample cannot be read, so the editor
/// pane is never empty and Dyon highlighting is always visible.
const SAMPLE_FALLBACK: &str = "\
// VisualRust Dyon sample\n\
fn main() {\n\
    /* greet the world */\n\
    name := \"VisualRust\"\n\
    count := 42\n\
    print(name + \" \" + str(count))\n\
}\n";

/// A file-backed or in-memory editor document.
///
/// The text is the version last loaded or saved; the live control may hold
/// unsaved edits on top of it, which is what the dirty indicator tracks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    /// `None` for the built-in sample; saving then needs a path that #29 adds.
    path: Option<PathBuf>,
    name: String,
    text: String,
}

impl Document {
    /// Reads `path` into a file-backed document.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Document> {
        let path = path.as_ref().to_path_buf();
        let text = std::fs::read_to_string(&path)?;
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        Ok(Document {
            path: Some(path),
            name,
            text,
        })
    }

    /// Builds an in-memory document with no path (the built-in fallback).
    pub fn untitled(text: impl Into<String>) -> Document {
        Document {
            path: None,
            name: "(sample)".to_owned(),
            text: text.into(),
        }
    }

    /// The name shown in the status bar.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The text as last loaded or saved.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Writes `text` to the backing file and adopts it as the saved version.
    pub fn write(&mut self, text: &str) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "the sample has no file path yet",
            ));
        };
        std::fs::write(path, text)?;
        self.text = text.to_owned();
        Ok(())
    }

    /// Re-reads the backing file, replacing the saved version.
    pub fn reload(&mut self) -> io::Result<()> {
        let Some(path) = self.path.clone() else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "the sample has no file path to reload",
            ));
        };
        self.text = std::fs::read_to_string(&path)?;
        Ok(())
    }
}

/// The document to open at startup and an optional note for the status bar.
pub struct Initial {
    pub document: Document,
    pub note: Option<String>,
}

/// Resolves and loads the startup document.
///
/// Precedence: an explicit `user` path (the CLI argument), then the entry of the
/// nearest `vrproj.toml` at or above `cwd`, then the on-disk sample, then a
/// built-in in-memory sample. A failed explicit path still opens the fallback so
/// the pane is never empty, and the reason lands in the status bar.
pub fn initial(user: Option<PathBuf>, cwd: &Path) -> Initial {
    if let Some(path) = user {
        return match Document::open(&path) {
            Ok(document) => Initial {
                document,
                note: None,
            },
            Err(error) => Initial {
                document: Document::untitled(SAMPLE_FALLBACK),
                note: Some(format!("Could not open {}: {error}", path.display())),
            },
        };
    }
    if let Some(path) = manifest_entry(cwd)
        && let Ok(document) = Document::open(&path)
    {
        return Initial {
            document,
            note: None,
        };
    }
    let sample = cwd.join(SAMPLE_RELATIVE);
    if let Ok(document) = Document::open(&sample) {
        return Initial {
            document,
            note: None,
        };
    }
    Initial {
        document: Document::untitled(SAMPLE_FALLBACK),
        note: Some("Showing the built-in Dyon sample".to_owned()),
    }
}

/// The first non-flag command-line argument, if any, as the path to open.
pub fn path_from_args(args: impl Iterator<Item = String>) -> Option<PathBuf> {
    args.filter(|arg| !arg.starts_with('-'))
        .map(PathBuf::from)
        .next()
}

/// Walks up from `cwd` for a manifest and returns its entry path.
fn manifest_entry(cwd: &Path) -> Option<PathBuf> {
    let mut dir = Some(cwd);
    while let Some(current) = dir {
        let manifest_path = current.join(MANIFEST_FILE);
        if manifest_path.is_file()
            && let Ok(manifest) = Manifest::load(&manifest_path)
        {
            return Some(current.join(&manifest.project.entry));
        }
        dir = current.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique scratch directory under the system temp dir.
    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("vr-ide-document-tests")
            .join(format!("{}-{name}", std::process::id()))
    }

    #[test]
    fn write_then_reload_adopts_the_new_text() {
        let dir = scratch("roundtrip");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("main.dyon");
        std::fs::write(&path, "fn main() {}\n").expect("seed the file");

        let mut document = Document::open(&path).expect("open");
        assert_eq!(document.name(), "main.dyon");
        document.write("fn main() { print(1) }\n").expect("write");

        // Change the file behind the document's back, then reload.
        std::fs::write(&path, "fn main() { print(2) }\n").expect("rewrite");
        document.reload().expect("reload");
        assert_eq!(document.text(), "fn main() { print(2) }\n");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_untitled_sample_cannot_save_or_reload() {
        let mut document = Document::untitled("fn main() {}\n");
        assert!(document.write("x").is_err());
        assert!(document.reload().is_err());
    }

    #[test]
    fn args_pick_the_first_non_flag() {
        let args = vec!["--theme".to_owned(), "app.dyon".to_owned()];
        assert_eq!(
            path_from_args(args.into_iter()),
            Some(PathBuf::from("app.dyon"))
        );
        assert_eq!(path_from_args(Vec::new().into_iter()), None);
    }

    #[test]
    fn a_missing_explicit_path_falls_back_with_a_note() {
        let initial = initial(Some(PathBuf::from("does-not-exist.dyon")), Path::new("."));
        assert!(initial.note.is_some());
        assert_eq!(initial.document.name(), "(sample)");
    }
}
