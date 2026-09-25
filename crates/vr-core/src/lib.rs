//! Project model, manifest, settings

mod fsutil;

pub mod manifest;
pub mod project;
pub mod settings;

#[cfg(test)]
pub(crate) mod test_support;

pub use project::{Project, ProjectOptions};
pub use settings::{EditorPrefs, RecentProject, Settings, SettingsStore, Theme};
