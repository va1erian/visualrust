//! Validation rules for a parsed [`Manifest`].
//!
//! Kept separate from the data shape so the rules can be read and tested on
//! their own; every failure is a typed [`ValidationError`], never a panic.

use std::collections::HashSet;
use std::path::{Component, Path};

use super::error::ValidationError;
use super::model::{Manifest, NamedPath, Project, ProjectKind, Route};

pub(crate) fn validate(manifest: &Manifest) -> Result<(), ValidationError> {
    check_project(&manifest.project)?;

    if manifest.project.kind == ProjectKind::Console && !manifest.forms.is_empty() {
        return Err(ValidationError::FormsNotAllowedForConsole);
    }

    check_named("form", &manifest.forms)?;
    check_named("module", &manifest.modules)?;
    check_named("asset", &manifest.assets)?;
    check_named("extension", &manifest.extensions)?;
    check_routes(manifest.project.kind, &manifest.routes)?;
    check_db(manifest)?;
    Ok(())
}

fn check_project(project: &Project) -> Result<(), ValidationError> {
    if project.name.trim().is_empty() {
        return Err(ValidationError::EmptyName {
            field: "project.name",
        });
    }
    if !is_valid_name(&project.name) {
        return Err(ValidationError::InvalidName {
            field: "project.name",
            name: project.name.clone(),
        });
    }
    if project.version.trim().is_empty() {
        return Err(ValidationError::EmptyVersion);
    }
    if project.entry.as_os_str().is_empty() {
        return Err(ValidationError::MissingEntry);
    }
    if !is_safe_relative(&project.entry) {
        return Err(ValidationError::EntryNotRelative(project.entry.clone()));
    }
    Ok(())
}

fn check_named(section: &'static str, items: &[NamedPath]) -> Result<(), ValidationError> {
    let mut seen: HashSet<&str> = HashSet::new();
    for item in items {
        if item.name.trim().is_empty() {
            return Err(ValidationError::EmptyName { field: section });
        }
        if !is_valid_name(&item.name) {
            return Err(ValidationError::InvalidName {
                field: section,
                name: item.name.clone(),
            });
        }
        if !is_safe_relative(&item.path) {
            return Err(ValidationError::UnsafePath {
                section,
                name: item.name.clone(),
                path: item.path.clone(),
            });
        }
        if !seen.insert(item.name.as_str()) {
            return Err(ValidationError::DuplicateName {
                section,
                name: item.name.clone(),
            });
        }
    }
    Ok(())
}

fn check_routes(kind: ProjectKind, routes: &[Route]) -> Result<(), ValidationError> {
    if !routes.is_empty() && kind != ProjectKind::Web {
        return Err(ValidationError::RoutesRequireWeb);
    }

    let mut names: HashSet<&str> = HashSet::new();
    let mut endpoints: HashSet<(super::model::HttpMethod, &str)> = HashSet::new();
    for route in routes {
        if route.name.trim().is_empty() {
            return Err(ValidationError::EmptyName { field: "route" });
        }
        if !is_valid_name(&route.name) {
            return Err(ValidationError::InvalidName {
                field: "route",
                name: route.name.clone(),
            });
        }
        if !route.path.starts_with('/') {
            return Err(ValidationError::InvalidRoutePath {
                path: route.path.clone(),
            });
        }
        if route.handler.trim().is_empty() {
            return Err(ValidationError::EmptyRouteHandler);
        }
        if !names.insert(route.name.as_str()) {
            return Err(ValidationError::DuplicateName {
                section: "route",
                name: route.name.clone(),
            });
        }
        if !endpoints.insert((route.method, route.path.as_str())) {
            return Err(ValidationError::DuplicateRoute {
                method: route.method,
                path: route.path.clone(),
            });
        }
    }
    Ok(())
}

fn check_db(manifest: &Manifest) -> Result<(), ValidationError> {
    let Some(db) = &manifest.db else {
        return Ok(());
    };
    if db.path.as_os_str().is_empty() {
        return Err(ValidationError::EmptyDbPath);
    }
    if !is_safe_relative(&db.path) {
        return Err(ValidationError::UnsafePath {
            section: "db",
            name: "path".to_owned(),
            path: db.path.clone(),
        });
    }
    Ok(())
}

/// Names double as generated symbols, so keep them to a portable identifier set.
fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Reject anything that could escape the project root: absolute paths, prefixes
/// and `..` traversal.
fn is_safe_relative(path: &Path) -> bool {
    if path.as_os_str().is_empty() {
        return false;
    }
    path.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}
