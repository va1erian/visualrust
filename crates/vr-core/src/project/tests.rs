use super::*;
use crate::manifest::ProjectKind;
use crate::test_support::TempDir;

fn desktop_options() -> ProjectOptions {
    ProjectOptions::new("demo", ProjectKind::Desktop).with_description("a test project")
}

#[test]
fn create_builds_the_directory_layout_and_manifest() {
    let dir = TempDir::new("create");
    let project = Project::create(dir.path(), desktop_options()).expect("create");

    assert!(dir.join(MANIFEST_FILE).is_file());
    assert!(dir.join(SRC_DIR).is_dir());
    assert!(dir.join(FORMS_DIR).is_dir());
    assert!(dir.join(ASSETS_DIR).is_dir());
    assert!(dir.join("src/main.dyon").is_file());
    assert_eq!(project.name(), "demo");
    assert_eq!(project.kind(), ProjectKind::Desktop);
}

#[test]
fn create_is_idempotent_and_keeps_the_first_manifest() {
    let dir = TempDir::create("idempotent").expect("temp dir");
    Project::create(dir.path(), desktop_options()).expect("first create");

    let again = Project::create(
        dir.path(),
        ProjectOptions::new("other", ProjectKind::Console),
    )
    .expect("second create");
    assert_eq!(again.name(), "demo");
}

#[test]
fn create_rejects_a_non_empty_directory() {
    let dir = TempDir::create("non-empty").expect("temp dir");
    std::fs::write(dir.join("stray.txt"), b"keep me").expect("write stray");

    let err = Project::create(dir.path(), desktop_options()).expect_err("must fail");
    assert!(matches!(err, ProjectError::DirectoryNotEmpty { .. }));
    assert!(dir.join("stray.txt").is_file());
}

#[test]
fn create_rolls_back_when_the_manifest_is_invalid() {
    let dir = TempDir::new("rollback");
    let options = ProjectOptions::new("bad name!", ProjectKind::Desktop);

    let err = Project::create(dir.path(), options).expect_err("must fail");
    assert!(matches!(err, ProjectError::Manifest(_)));
    assert!(!dir.path().exists(), "no partial project left behind");
}

#[test]
fn create_rejects_a_file_at_the_root() {
    let dir = TempDir::create("root-file").expect("temp dir");
    let file = dir.join("not-a-dir");
    std::fs::write(&file, b"").expect("write file");

    let err = Project::create(&file, desktop_options()).expect_err("must fail");
    assert!(matches!(err, ProjectError::RootIsFile { .. }));
}

#[test]
fn open_loads_and_heals_missing_directories() {
    let dir = TempDir::new("heal");
    Project::create(dir.path(), desktop_options()).expect("create");
    std::fs::remove_dir_all(dir.join(ASSETS_DIR)).expect("drop assets dir");

    let project = Project::open(dir.path()).expect("open");
    assert_eq!(project.name(), "demo");
    assert!(dir.join(ASSETS_DIR).is_dir(), "open recreates the layout");
}

#[test]
fn open_missing_manifest_is_a_typed_error() {
    let dir = TempDir::create("missing").expect("temp dir");
    let err = Project::open(dir.path()).expect_err("must fail");
    assert!(matches!(
        err,
        ProjectError::Manifest(crate::manifest::ManifestError::Read { .. })
    ));
}

#[test]
fn add_module_persists_the_manifest_and_file() {
    let dir = TempDir::new("add-module");
    let mut project = Project::create(dir.path(), desktop_options()).expect("create");

    project
        .add_module("util", "src/util.dyon")
        .expect("add module");
    assert!(dir.join("src/util.dyon").is_file());

    let reopened = Project::open(dir.path()).expect("reopen");
    assert_eq!(reopened.manifest().modules.len(), 1);
    assert_eq!(reopened.manifest().modules[0].name, "util");
}

#[test]
fn add_operations_are_idempotent() {
    let dir = TempDir::new("add-twice");
    let mut project = Project::create(dir.path(), desktop_options()).expect("create");

    project.add_module("util", "src/util.dyon").expect("first");
    project.add_module("util", "src/util.dyon").expect("second");

    project.add_form("main", "forms/main.vrform").expect("form");
    project
        .add_form("main", "forms/main.vrform")
        .expect("form again");

    project.add_file("icon", "assets/app.ico").expect("asset");
    project
        .add_file("icon", "assets/app.ico")
        .expect("asset again");

    assert_eq!(project.manifest().modules.len(), 1);
    assert_eq!(project.manifest().forms.len(), 1);
    assert_eq!(project.manifest().assets.len(), 1);
    assert!(dir.join("forms/main.vrform").is_file());
    assert!(dir.join("assets/app.ico").is_file());
}

#[test]
fn adding_a_name_at_a_second_path_is_rejected() {
    let dir = TempDir::new("duplicate");
    let mut project = Project::create(dir.path(), desktop_options()).expect("create");
    project.add_module("util", "src/util.dyon").expect("first");

    let err = project
        .add_module("util", "src/other.dyon")
        .expect_err("must fail");
    assert!(matches!(err, ProjectError::DuplicateEntry { .. }));
    assert!(!dir.join("src/other.dyon").exists());
    assert_eq!(project.manifest().modules.len(), 1);
}

#[test]
fn remove_deletes_the_entry_and_its_file() {
    let dir = TempDir::new("remove");
    let mut project = Project::create(dir.path(), desktop_options()).expect("create");
    project.add_module("util", "src/util.dyon").expect("add");

    project.remove("util").expect("remove");
    assert!(!dir.join("src/util.dyon").exists());
    assert!(project.manifest().modules.is_empty());

    let reopened = Project::open(dir.path()).expect("reopen");
    assert!(reopened.manifest().modules.is_empty());
}

#[test]
fn remove_is_idempotent_for_unknown_names() {
    let dir = TempDir::new("remove-unknown");
    let mut project = Project::create(dir.path(), desktop_options()).expect("create");
    project.remove("ghost").expect("no-op");
    project.remove("ghost").expect("still no-op");
}
