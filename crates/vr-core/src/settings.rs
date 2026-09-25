//! User settings persisted under `%APPDATA%/VisualRust`.
//!
//! [`SettingsStore`] owns the directory; tests inject a temp root through
//! [`SettingsStore::at`] so the real `%APPDATA%` is never touched.

mod error;

#[cfg(test)]
mod tests;

pub use error::SettingsError;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::fsutil;

/// Directory name under `%APPDATA%` that holds all VisualRust state.
pub const APP_DIR: &str = "VisualRust";
/// Filename of the persisted settings document.
pub const SETTINGS_FILE: &str = "settings.toml";
/// How many recent projects to remember.
pub const MAX_RECENTS: usize = 10;

/// Everything the IDE remembers between sessions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub recent_projects: Vec<RecentProject>,
    pub editor: EditorPrefs,
}

impl Settings {
    /// Moves `path` to the front of the recents, refreshing its timestamp.
    pub fn record_recent(&mut self, name: impl Into<String>, path: impl Into<PathBuf>) {
        let path = path.into();
        self.recent_projects
            .retain(|entry| !same_path(&entry.path, &path));
        self.recent_projects.insert(
            0,
            RecentProject {
                name: name.into(),
                path,
                last_opened: now_unix(),
            },
        );
        self.recent_projects.truncate(MAX_RECENTS);
    }

    pub fn clear_recents(&mut self) {
        self.recent_projects.clear();
    }

    /// Rejects values that would otherwise reach disk and quietly misbehave.
    pub fn validate(&self) -> Result<(), SettingsError> {
        if !(1..=200).contains(&self.editor.font_size) {
            return Err(SettingsError::FontSizeOutOfRange(self.editor.font_size));
        }
        if !(1..=16).contains(&self.editor.tab_width) {
            return Err(SettingsError::TabWidthOutOfRange(self.editor.tab_width));
        }
        if self.editor.font_family.trim().is_empty() {
            return Err(SettingsError::EmptyFontFamily);
        }
        for entry in &self.recent_projects {
            if entry.path.as_os_str().is_empty() {
                return Err(SettingsError::EmptyRecentPath);
            }
        }
        Ok(())
    }
}

/// One remembered project, most recent first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecentProject {
    pub name: String,
    pub path: PathBuf,
    /// Unix seconds, so settings stay free of a date-time dependency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_opened: Option<u64>,
}

/// Editor look-and-feel preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EditorPrefs {
    pub font_family: String,
    pub font_size: u32,
    pub tab_width: u32,
    pub use_spaces: bool,
    pub theme: Theme,
}

impl Default for EditorPrefs {
    fn default() -> Self {
        Self {
            font_family: "Consolas".to_owned(),
            font_size: 11,
            tab_width: 4,
            use_spaces: true,
            theme: Theme::System,
        }
    }
}

/// Which color scheme the editor follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

/// Handle on the settings directory.
#[derive(Debug, Clone)]
pub struct SettingsStore {
    root: PathBuf,
}

impl SettingsStore {
    /// Locates `%APPDATA%/VisualRust` for the current user.
    pub fn for_user() -> Result<Self, SettingsError> {
        let appdata = std::env::var_os("APPDATA").ok_or(SettingsError::MissingAppData)?;
        Ok(Self {
            root: PathBuf::from(appdata).join(APP_DIR),
        })
    }

    /// Points the store at an explicit root; used by tests and portable mode.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn settings_path(&self) -> PathBuf {
        self.root.join(SETTINGS_FILE)
    }

    /// Loads settings, falling back to defaults when nothing is stored yet.
    pub fn load(&self) -> Result<Settings, SettingsError> {
        let path = self.settings_path();
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(toml::from_str(&text)?),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(source) => Err(SettingsError::Read { path, source }),
        }
    }

    /// Validates then writes settings atomically, creating the directory.
    pub fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
        settings.validate()?;
        std::fs::create_dir_all(&self.root)
            .map_err(|source| SettingsError::write(&self.root, source))?;
        let text = toml::to_string_pretty(settings)?;
        let path = self.settings_path();
        fsutil::write_atomic(&path, text.as_bytes())
            .map_err(|source| SettingsError::write(path, source))
    }
}

fn now_unix() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .ok()
}

/// Windows paths are case-insensitive, so recents must dedupe that way too.
fn same_path(a: &Path, b: &Path) -> bool {
    a == b || a.as_os_str().eq_ignore_ascii_case(b.as_os_str())
}
