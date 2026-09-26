//! Free helpers shared by the shell's modules: CLI and environment parsing, the
//! Scintilla notification mapping and timer arming. Kept out of `app/mod.rs` so
//! the app module stays under the file-size limit.

use std::path::{Path, PathBuf};

use vr_scintilla::Scn;
use xui::prelude::*;

use super::{AUTOCLOSE_ENV, RESIZE_ENV};
use crate::msg::Msg;

/// Splits the CLI argument into a project start directory and an optional file.
pub(crate) fn split_argument(path: Option<PathBuf>, cwd: &Path) -> (PathBuf, Option<PathBuf>) {
    match path {
        Some(path) if path.is_dir() => (path, None),
        Some(path) => {
            let parent = path
                .parent()
                .filter(|dir| !dir.as_os_str().is_empty())
                .map(Path::to_path_buf)
                .unwrap_or_else(|| cwd.to_path_buf());
            (parent, Some(path))
        }
        None => (cwd.to_path_buf(), None),
    }
}

/// Whether an environment variable is set to a truthy value.
pub(crate) fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| value != "0" && !value.is_empty())
}

/// The millisecond value of an environment variable, if it parses.
fn env_millis(name: &str) -> Option<u32> {
    std::env::var(name).ok()?.parse::<u32>().ok()
}

/// Maps a Scintilla notification to the app's message, or `None` to ignore it.
///
/// A modification and both save-point transitions all ask for a dirty
/// recomputation. Scintilla also raises `SCN_MODIFIED` for style changes, which
/// is why the dirty flag is a text comparison rather than "was modified".
pub(crate) fn map_scn(scn: Scn) -> Option<Msg> {
    match scn {
        Scn::Modified { .. } | Scn::SavePointLeft | Scn::SavePointReached => {
            Some(Msg::DocumentChanged)
        }
        _ => None,
    }
}

/// A palette for the window; `dark` selects [`Theme::dark`].
pub(crate) fn palette(dark: bool) -> Theme {
    if dark { Theme::dark() } else { Theme::light() }
}

/// Arms the autoclose and design-resize timers named by their environment
/// variables. One `WM_TIMER` mapping dispatches both, since only one can be
/// installed per window.
pub(crate) fn arm_timers(ui: &mut Ui<Msg>) {
    let autoclose = env_millis(AUTOCLOSE_ENV).and_then(|millis| ui.set_timer(millis).ok());
    let resize = env_millis(RESIZE_ENV).and_then(|millis| ui.set_timer(millis).ok());
    if autoclose.is_none() && resize.is_none() {
        return;
    }
    ui.on_timer(move |fired| {
        if Some(fired) == autoclose {
            Some(Msg::AutoClose)
        } else if Some(fired) == resize {
            Some(Msg::ResizeFormToPane)
        } else {
            None
        }
    });
}
