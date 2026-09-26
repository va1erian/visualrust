//! The project explorer's view model.
//!
//! Loading a project is a filesystem side effect, so it lives here rather than
//! in [`IdeState`](crate::IdeState): the pure reducer only stores the resulting
//! item list. The list is flat — a project's files and forms with their names —
//! which is all the prototype's [`TreeView`](xui::TreeView) needs. The tree's
//! key is the item's index into that list.
//!
//! A directory without a `vrproj.toml` still opens: the prototype falls back to
//! a small built-in sample form plus the on-disk Dyon example, so the window
//! always shows an explorer, an editor and a designer.

use std::path::{Path, PathBuf};

use vr_core::manifest::Manifest;
use vr_forms::model::{CheckBoxProps, RangeProps};
use vr_forms::{Bounds, Control, ControlKind, Dip, Form, Size};
use xui::prelude::*;

/// The manifest filename that marks a project root.
const MANIFEST_FILE: &str = "vrproj.toml";
/// How far up from the working directory to look for a manifest, in levels.
const ROOT_SEARCH_DEPTH: usize = 8;
/// The on-disk Dyon sample opened when no project provides an entry.
pub const SAMPLE_RELATIVE: &str = "examples/hello-window/src/main.dyon";

/// What an explorer item opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    /// A `.vrform`; selecting it switches the central pane to the designer.
    Form,
    /// A Dyon source; selecting it opens it in the editor.
    Source,
}

/// One row of the explorer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplorerItem {
    /// The name shown in the tree.
    pub name: String,
    /// The file backing the item; `None` for the built-in sample.
    pub path: Option<PathBuf>,
    /// What selecting the item does.
    pub kind: ItemKind,
}

/// A loaded project (or the built-in fallback) and its explorer rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loaded {
    /// The directory the project was resolved from.
    pub root: PathBuf,
    /// The project name, shown in the status line.
    pub name: String,
    /// The flat explorer list, in display order.
    pub items: Vec<ExplorerItem>,
    /// A one-line note for the output pane: what was loaded, or why the
    /// fallback was used.
    pub note: String,
}

/// The [`TreeModel`] the explorer's `TreeView` reads: a flat list of items.
pub struct ExplorerModel {
    items: Vec<ExplorerItem>,
}

impl ExplorerModel {
    /// A model over `items`.
    pub fn new(items: Vec<ExplorerItem>) -> ExplorerModel {
        ExplorerModel { items }
    }
}

impl TreeModel for ExplorerModel {
    type Key = usize;

    fn children(&self, parent: Option<&usize>) -> Vec<Node<usize>> {
        // The prototype lists every file at the root; nested folders are a
        // later issue, so a branch request never has children.
        if parent.is_some() {
            return Vec::new();
        }
        self.items
            .iter()
            .enumerate()
            .map(|(index, item)| Node::leaf(index, item.name.clone()))
            .collect()
    }
}

/// Resolves the project from `start`, walking up for a `vrproj.toml`.
///
/// On a manifest the project's declared forms/modules are listed first, then
/// any `.vrform`/`.dyon` files found in the standard directories. Without one
/// the built-in sample is returned with the reason in [`Loaded::note`].
pub fn load(start: &Path) -> Loaded {
    let Some(root) = find_root(start) else {
        return fallback(start, "No vrproj.toml found; showing the built-in sample");
    };
    let manifest_path = root.join(MANIFEST_FILE);
    match Manifest::load(&manifest_path) {
        Ok(manifest) => {
            let items = collect_items(&root, &manifest);
            let name = manifest.project.name.clone();
            Loaded {
                note: format!("Loaded project {name}"),
                root,
                name,
                items,
            }
        }
        Err(error) => fallback(
            start,
            &format!("Could not read {}: {error}", manifest_path.display()),
        ),
    }
}

/// The nearest ancestor (including `start`) that holds a manifest.
fn find_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    for _ in 0..ROOT_SEARCH_DEPTH {
        let dir = current?;
        if dir.join(MANIFEST_FILE).is_file() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

/// Declared files plus anything found in `forms/` and `src/`, de-duplicated by
/// path so a declared entry is not listed twice.
fn collect_items(root: &Path, manifest: &Manifest) -> Vec<ExplorerItem> {
    let mut items: Vec<ExplorerItem> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();

    let mut push = |name: String, path: Option<PathBuf>, kind: ItemKind| {
        if let Some(path) = &path
            && seen.contains(path)
        {
            return;
        }
        if let Some(path) = &path {
            seen.push(path.clone());
        }
        items.push(ExplorerItem { name, path, kind });
    };

    for form in &manifest.forms {
        push(
            form.name.clone(),
            Some(root.join(&form.path)),
            ItemKind::Form,
        );
    }
    for module in &manifest.modules {
        push(
            module.name.clone(),
            Some(root.join(&module.path)),
            ItemKind::Source,
        );
    }
    if !manifest.project.entry.as_os_str().is_empty() {
        push(
            entry_name(&manifest.project.entry),
            Some(root.join(&manifest.project.entry)),
            ItemKind::Source,
        );
    }

    for path in scan_dir(&root.join("forms"), "vrform") {
        push(file_stem(&path), Some(path), ItemKind::Form);
    }
    for path in scan_dir(&root.join("src"), "dyon") {
        push(file_stem(&path), Some(path), ItemKind::Source);
    }
    items
}

/// The file name of an entry path, for display.
fn entry_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// A path's file stem, falling back to the whole name.
fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Every file under `dir` with `extension`, ignoring read errors.
fn scan_dir(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == extension))
        .collect();
    found.sort();
    found
}

/// The built-in sample project, used when no manifest is found.
fn fallback(start: &Path, note: &str) -> Loaded {
    let sample_source = start.join(SAMPLE_RELATIVE);
    let source = if sample_source.is_file() {
        ExplorerItem {
            name: file_stem(&sample_source),
            path: Some(sample_source),
            kind: ItemKind::Source,
        }
    } else {
        ExplorerItem {
            name: "main.dyon".to_owned(),
            path: None,
            kind: ItemKind::Source,
        }
    };
    Loaded {
        root: start.to_path_buf(),
        name: "sample".to_owned(),
        items: vec![
            ExplorerItem {
                name: "SampleForm".to_owned(),
                path: None,
                kind: ItemKind::Form,
            },
            source,
        ],
        note: note.to_owned(),
    }
}

/// The demo form shown when a project has no `.vrform` of its own.
pub fn sample_form() -> Form {
    Form {
        name: "SampleForm".to_owned(),
        size: Size {
            width: Dip::new(360.0),
            height: Dip::new(260.0),
        },
        controls: vec![
            control(
                ControlKind::Label,
                "Title",
                24.0,
                20.0,
                300.0,
                28.0,
                "Hello, VisualRust",
            ),
            control(
                ControlKind::Label,
                "Prompt",
                24.0,
                60.0,
                120.0,
                20.0,
                "Your name",
            ),
            control(
                ControlKind::Edit(Default::default()),
                "Name",
                24.0,
                84.0,
                280.0,
                28.0,
                "",
            ),
            control(
                ControlKind::Button,
                "Greet",
                24.0,
                124.0,
                96.0,
                30.0,
                "Greet",
            ),
            control(
                ControlKind::CheckBox(CheckBoxProps::default()),
                "Subscribe",
                24.0,
                168.0,
                240.0,
                24.0,
                "Subscribe to updates",
            ),
            control(
                ControlKind::ProgressBar(RangeProps {
                    min: 0.0,
                    max: 100.0,
                    value: 60.0,
                }),
                "Progress",
                24.0,
                204.0,
                280.0,
                18.0,
                "",
            ),
        ],
    }
}

/// A sample-project Dyon document, used when the on-disk sample is missing.
pub const SAMPLE_SOURCE: &str = "\
// VisualRust Dyon sample
fn main() {
    /* greet the world */
    name := \"VisualRust\"
    count := 42
    print(name + \" \" + str(count))
}
";

/// One named control at a form-relative position.
fn control(
    kind: ControlKind,
    name: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    text: &str,
) -> Control {
    Control {
        kind,
        name: name.to_owned(),
        bounds: Bounds {
            x: Dip::new(x),
            y: Dip::new(y),
            width: Dip::new(width),
            height: Dip::new(height),
        },
        text: text.to_owned(),
        enabled: true,
        visible: true,
        tooltip: None,
        anchor: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique scratch directory under the system temp dir.
    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("vr-ide-explorer-tests")
            .join(format!("{}-{name}", std::process::id()))
    }

    #[test]
    fn a_directory_without_a_manifest_falls_back_to_the_sample() {
        let dir = scratch("no-manifest");
        let _ = std::fs::create_dir_all(&dir);
        let loaded = load(&dir);
        assert_eq!(loaded.name, "sample");
        assert!(loaded.items.iter().any(|item| item.kind == ItemKind::Form));
        assert!(
            loaded
                .items
                .iter()
                .any(|item| item.kind == ItemKind::Source)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_manifest_lists_its_entry_and_scanned_forms() {
        let dir = scratch("with-manifest");
        let _ = std::fs::create_dir_all(dir.join("src"));
        let _ = std::fs::create_dir_all(dir.join("forms"));
        std::fs::write(
            dir.join(MANIFEST_FILE),
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\ntype = \"desktop\"\nentry = \"src/main.dyon\"\n",
        )
        .expect("write manifest");
        std::fs::write(dir.join("src/main.dyon"), "fn main() {}\n").expect("write entry");
        std::fs::write(dir.join("forms/app.vrform"), "{}").expect("write form");

        let loaded = load(&dir);
        assert_eq!(loaded.name, "demo");
        assert!(loaded.items.iter().any(|item| item.name == "main.dyon"));
        assert!(loaded.items.iter().any(|item| item.name == "app"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_sample_form_is_valid() {
        // A built-in sample that fails validation would make the designer
        // unavailable for every project without a form of its own.
        sample_form().validate().expect("sample form validates");
    }
}
