use super::*;
use crate::test_support::TempDir;

#[test]
fn missing_settings_load_as_defaults() {
    let dir = TempDir::create("defaults").expect("temp dir");
    let store = SettingsStore::at(dir.path());

    let settings = store.load().expect("load");
    assert_eq!(settings, Settings::default());
    assert_eq!(settings.editor.font_family, "Consolas");
    assert_eq!(settings.editor.theme, Theme::System);
}

#[test]
fn save_then_load_round_trips() {
    let dir = TempDir::create("round-trip").expect("temp dir");
    let store = SettingsStore::at(dir.path());

    let mut settings = Settings::default();
    settings.editor.font_size = 14;
    settings.editor.theme = Theme::Dark;
    settings.record_recent("notes", dir.join("notes"));

    store.save(&settings).expect("save");
    let loaded = store.load().expect("load");
    assert_eq!(settings, loaded);
}

#[test]
fn the_injected_root_keeps_writes_out_of_appdata() {
    let dir = TempDir::create("injected").expect("temp dir");
    let store = SettingsStore::at(dir.path());
    store.save(&Settings::default()).expect("save");

    assert!(store.settings_path().starts_with(dir.path()));
    assert!(store.settings_path().is_file());
}

#[test]
fn record_recent_moves_duplicates_to_the_front() {
    let mut settings = Settings::default();
    settings.record_recent("a", "C:/work/a");
    settings.record_recent("b", "C:/work/b");
    settings.record_recent("a", "c:/WORK/A");

    assert_eq!(settings.recent_projects.len(), 2);
    assert_eq!(settings.recent_projects[0].name, "a");
}

#[test]
fn record_recent_caps_the_list() {
    let mut settings = Settings::default();
    for index in 0..MAX_RECENTS + 5 {
        settings.record_recent(format!("p{index}"), format!("C:/work/p{index}"));
    }

    assert_eq!(settings.recent_projects.len(), MAX_RECENTS);
    assert_eq!(settings.recent_projects[0].name, "p14");
}

#[test]
fn clear_recents_empties_the_list() {
    let mut settings = Settings::default();
    settings.record_recent("a", "C:/work/a");
    settings.clear_recents();
    assert!(settings.recent_projects.is_empty());
}

#[test]
fn invalid_editor_prefs_are_rejected_before_writing() {
    let dir = TempDir::create("invalid").expect("temp dir");
    let store = SettingsStore::at(dir.path());

    let mut settings = Settings::default();
    settings.editor.font_size = 0;
    assert!(matches!(
        store.save(&settings),
        Err(SettingsError::FontSizeOutOfRange(0))
    ));
    assert!(!store.settings_path().exists(), "nothing written on reject");

    settings.editor.font_size = 11;
    settings.editor.tab_width = 99;
    assert!(matches!(
        store.save(&settings),
        Err(SettingsError::TabWidthOutOfRange(99))
    ));

    settings.editor.tab_width = 4;
    settings.editor.font_family = "  ".to_owned();
    assert!(matches!(
        store.save(&settings),
        Err(SettingsError::EmptyFontFamily)
    ));
}

#[test]
fn malformed_settings_are_a_typed_error() {
    let dir = TempDir::create("malformed").expect("temp dir");
    let store = SettingsStore::at(dir.path());
    std::fs::write(store.settings_path(), b"editor = [").expect("write malformed");

    assert!(matches!(store.load(), Err(SettingsError::Parse(_))));
}
