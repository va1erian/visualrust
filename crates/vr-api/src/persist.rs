//! Persistence for endpoints.
//!
//! # Choice
//!
//! Endpoints are stored in the project manifest's `[[routes]]` table rather
//! than a sibling file. The manifest is already the runtime's source of truth
//! for handler routing (#86) and `vr-core` validates it, so a second file would
//! fork that source and let the editor and server disagree. The API-design-only
//! fields (`description`, `params`, `response`) ride along as optional keys and
//! are omitted when unset, so existing manifests are unchanged.

use std::path::Path;

use vr_core::manifest::Manifest;

use crate::error::ApiError;
use crate::model::Endpoint;

/// Reads the endpoints a parsed manifest declares, preserving order.
pub fn endpoints_from_manifest(manifest: &Manifest) -> Vec<Endpoint> {
    manifest
        .routes
        .iter()
        .cloned()
        .map(Endpoint::from_route)
        .collect()
}

/// Replaces the manifest's routes with `endpoints`.
///
/// Other sections (`forms`, `db`, ...) are untouched, so an API edit never
/// rewrites unrelated project configuration.
pub fn apply_endpoints(manifest: &mut Manifest, endpoints: &[Endpoint]) {
    manifest.routes = endpoints
        .iter()
        .cloned()
        .map(Endpoint::into_route)
        .collect();
}

/// Loads and maps the endpoints declared in a `vrproj.toml`.
pub fn load_endpoints(path: impl AsRef<Path>) -> Result<Vec<Endpoint>, ApiError> {
    let manifest = Manifest::load(path)?;
    Ok(endpoints_from_manifest(&manifest))
}

/// Loads the manifest, swaps in `endpoints`, and writes it back.
///
/// Loading first keeps the unrelated sections that the caller does not model.
pub fn save_endpoints(path: impl AsRef<Path>, endpoints: &[Endpoint]) -> Result<(), ApiError> {
    let path = path.as_ref();
    let mut manifest = Manifest::load(path)?;
    apply_endpoints(&mut manifest, endpoints);
    manifest.save(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{apply_endpoints, endpoints_from_manifest, load_endpoints, save_endpoints};
    use crate::model::Endpoint;
    use vr_core::manifest::HttpMethod;
    use vr_core::manifest::{
        Manifest, ParamLocation, Project, ProjectKind, RouteParam, SampleResponse,
    };

    fn web_manifest() -> Manifest {
        Manifest {
            project: Project {
                name: "todo-web".to_owned(),
                version: "0.1.0".to_owned(),
                kind: ProjectKind::Web,
                entry: "src/main.dyon".into(),
                description: None,
            },
            forms: Vec::new(),
            modules: Vec::new(),
            assets: Vec::new(),
            extensions: Vec::new(),
            routes: Vec::new(),
            db: None,
        }
    }

    fn endpoints() -> Vec<Endpoint> {
        let mut show = Endpoint::new("show", HttpMethod::Get, "/todos/:id", "show_todo");
        show.set_description("fetch one");
        show.add_param(RouteParam {
            name: "id".to_owned(),
            location: ParamLocation::Path,
            required: true,
            description: None,
        });
        show.set_response(SampleResponse {
            status: 200,
            body: Some("{\"id\":1}".to_owned()),
            content_type: "application/json".to_owned(),
        });

        vec![Endpoint::new("index", HttpMethod::Get, "/", "index"), show]
    }

    #[test]
    fn endpoints_round_trip_through_the_manifest() {
        let dir = std::env::temp_dir().join(format!("vr-api-persist-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("vrproj.toml");

        let mut manifest = web_manifest();
        apply_endpoints(&mut manifest, &endpoints());
        manifest.save(&path).expect("save seeded manifest");

        let loaded = load_endpoints(&path).expect("load endpoints");
        assert_eq!(loaded, endpoints());
        assert_eq!(endpoints_from_manifest(&manifest), endpoints());

        std::fs::remove_dir_all(&dir).expect("clean temp dir");
    }

    #[test]
    fn save_replaces_routes_but_keeps_other_sections() {
        let dir = std::env::temp_dir().join(format!("vr-api-replace-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("vrproj.toml");

        let mut manifest = web_manifest();
        manifest.db = Some(vr_core::manifest::DbConfig {
            path: "data/todos.db".into(),
        });
        manifest.save(&path).expect("save seeded manifest");

        save_endpoints(&path, &endpoints()).expect("save endpoints");
        let reloaded = Manifest::load(&path).expect("reload");
        assert!(reloaded.db.is_some(), "db section must survive an API edit");
        assert_eq!(endpoints_from_manifest(&reloaded), endpoints());

        std::fs::remove_dir_all(&dir).expect("clean temp dir");
    }
}
