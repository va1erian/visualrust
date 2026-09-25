use std::path::{Path, PathBuf};

use super::*;

fn web_manifest() -> Manifest {
    Manifest {
        project: Project {
            name: "todo-web".to_owned(),
            version: "0.1.0".to_owned(),
            kind: ProjectKind::Web,
            entry: "src/main.dyon".into(),
            description: Some("Route and database demo".to_owned()),
        },
        forms: Vec::new(),
        modules: vec![NamedPath {
            name: "handlers".to_owned(),
            path: "src/handlers.dyon".into(),
        }],
        assets: Vec::new(),
        extensions: Vec::new(),
        routes: vec![
            Route {
                name: "index".to_owned(),
                method: HttpMethod::Get,
                path: "/".to_owned(),
                handler: "index".to_owned(),
            },
            Route {
                name: "create".to_owned(),
                method: HttpMethod::Post,
                path: "/todos".to_owned(),
                handler: "create".to_owned(),
            },
        ],
        db: Some(DbConfig {
            path: "data/todos.db".into(),
        }),
    }
}

#[test]
fn web_manifest_round_trips_through_toml() {
    let manifest = web_manifest();
    let text = manifest.to_toml().expect("serialize");

    assert!(text.contains("[[routes]]"));
    assert!(text.contains("[db]"));
    assert!(text.contains("method = \"POST\""));

    let parsed = Manifest::from_toml(&text).expect("parse");
    assert_eq!(manifest, parsed);
}

#[test]
fn save_and_load_round_trips_through_a_file() {
    let dir = std::env::temp_dir().join(format!("vr-core-manifest-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("vrproj.toml");

    let manifest = web_manifest();
    manifest.save(&path).expect("save");
    let loaded = Manifest::load(&path).expect("load");

    assert_eq!(manifest, loaded);
    std::fs::remove_dir_all(&dir).expect("clean temp dir");
}

#[test]
fn checked_in_examples_load_and_validate() {
    for (stem, kind) in [
        ("desktop", ProjectKind::Desktop),
        ("console", ProjectKind::Console),
        ("web", ProjectKind::Web),
    ] {
        let path = examples_dir().join(format!("{stem}.vrproj.toml"));
        let manifest = Manifest::load(&path).unwrap_or_else(|err| panic!("{stem} example: {err}"));
        assert_eq!(manifest.project.kind, kind, "{stem} example kind");
    }
}

#[test]
fn web_example_declares_routes_and_a_database() {
    let manifest = Manifest::load(examples_dir().join("web.vrproj.toml")).expect("web example");
    assert!(!manifest.routes.is_empty());
    assert!(manifest.db.is_some());
}

#[test]
fn missing_entry_is_reported() {
    let mut manifest = web_manifest();
    manifest.project.entry = PathBuf::new();
    assert_eq!(manifest.validate(), Err(ValidationError::MissingEntry));
}

#[test]
fn missing_entry_field_is_a_typed_error() {
    let text = r#"
[project]
name = "tiny"
version = "0.1.0"
type = "console"
"#;
    assert!(matches!(
        Manifest::from_toml(text),
        Err(ManifestError::Invalid(ValidationError::MissingEntry))
    ));
}

#[test]
fn duplicate_module_names_are_reported() {
    let mut manifest = web_manifest();
    manifest.modules.push(NamedPath {
        name: "handlers".to_owned(),
        path: "src/other.dyon".into(),
    });
    assert_eq!(
        manifest.validate(),
        Err(ValidationError::DuplicateName {
            section: "module",
            name: "handlers".to_owned(),
        })
    );
}

#[test]
fn duplicate_route_endpoints_are_reported() {
    let mut manifest = web_manifest();
    manifest.routes.push(Route {
        name: "index-again".to_owned(),
        method: HttpMethod::Get,
        path: "/".to_owned(),
        handler: "index_again".to_owned(),
    });
    assert!(matches!(
        manifest.validate(),
        Err(ValidationError::DuplicateRoute { .. })
    ));
}

#[test]
fn routes_are_rejected_outside_web_projects() {
    let mut manifest = web_manifest();
    manifest.project.kind = ProjectKind::Desktop;
    assert_eq!(manifest.validate(), Err(ValidationError::RoutesRequireWeb));
}

#[test]
fn console_projects_cannot_declare_forms() {
    let mut manifest = web_manifest();
    manifest.project.kind = ProjectKind::Console;
    manifest.routes.clear();
    manifest.db = None;
    manifest.forms.push(NamedPath {
        name: "main".to_owned(),
        path: "forms/main.vrform".into(),
    });
    assert_eq!(
        manifest.validate(),
        Err(ValidationError::FormsNotAllowedForConsole)
    );
}

#[test]
fn absolute_entry_is_rejected() {
    let mut manifest = web_manifest();
    manifest.project.entry = PathBuf::from(r"C:\projects\app\main.dyon");
    assert_eq!(
        manifest.validate(),
        Err(ValidationError::EntryNotRelative(manifest.project.entry))
    );
}

#[test]
fn parent_dir_paths_are_rejected() {
    let mut manifest = web_manifest();
    manifest.modules[0].path = "../outside.dyon".into();
    assert!(matches!(
        manifest.validate(),
        Err(ValidationError::UnsafePath { .. })
    ));
}

#[test]
fn malformed_toml_is_an_error_not_a_panic() {
    assert!(matches!(
        Manifest::from_toml("project = ["),
        Err(ManifestError::Parse(_))
    ));
}

#[test]
fn unknown_fields_are_rejected() {
    let text = r#"
[project]
name = "tiny"
version = "0.1.0"
type = "console"
entry = "src/main.dyon"
bogus = true
"#;
    assert!(matches!(
        Manifest::from_toml(text),
        Err(ManifestError::Parse(_))
    ));
}

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples")
}
