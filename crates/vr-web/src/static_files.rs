//! Static file serving with path-traversal protection.
//!
//! A [`static_files`] handler maps a URL prefix onto a directory and serves the
//! file named by the rest of the path. The path is rejected unless every
//! component is a plain name, so `..` (percent-encoded or not) can never escape
//! the root, and the resolved file is canonicalised and checked against the root
//! as a second line of defence.

use std::path::{Component, Path, PathBuf};

use crate::handler::Handler;
use crate::method::Method;
use crate::request::Request;
use crate::response::{Response, StatusCode};

/// Builds a handler that serves files under `root` at `url_prefix`.
///
/// A request for the prefix itself serves `index.html`. Files are served for
/// `GET`; `HEAD` returns the headers without a body. Anything missing or unsafe
/// is a `404`.
pub fn static_files(root: impl Into<PathBuf>, url_prefix: impl Into<String>) -> impl Handler {
    let root = root.into();
    let prefix = normalize_prefix(&url_prefix.into());
    move |request: &Request| serve(&root, &prefix, request)
}

/// Strips a trailing slash so `/static/` and `/static` are the same prefix.
fn normalize_prefix(prefix: &str) -> String {
    prefix.trim_end_matches('/').to_owned()
}

fn serve(root: &Path, prefix: &str, request: &Request) -> Response {
    match request.method {
        Method::Get | Method::Head => {}
        _ => {
            return Response::text(StatusCode::METHOD_NOT_ALLOWED, "method not allowed")
                .with_header("allow", "GET, HEAD");
        }
    }

    let not_found = || Response::text(StatusCode::NOT_FOUND, "not found");
    let Some(rest) = request.path.strip_prefix(prefix) else {
        return not_found();
    };
    // Reject `/staticfoo` matching a `/static` prefix.
    if !rest.is_empty() && !rest.starts_with('/') {
        return not_found();
    }

    let relative = rest.trim_start_matches('/');
    let relative = if relative.is_empty() {
        "index.html"
    } else {
        relative
    };
    let relative = Path::new(relative);
    if !is_safe(relative) {
        return not_found();
    }

    let Ok(root_canonical) = std::fs::canonicalize(root) else {
        return not_found();
    };
    let Ok(file) = std::fs::canonicalize(root.join(relative)) else {
        return not_found();
    };
    if !file.starts_with(&root_canonical) || !file.is_file() {
        return not_found();
    }

    match std::fs::read(&file) {
        Ok(bytes) => {
            let mut response = Response::bytes(StatusCode::OK, content_type_for(relative), bytes);
            if request.method == Method::Head {
                let length = response.body.len().to_string();
                response.body.clear();
                response.headers.insert("content-length", length);
            }
            response
        }
        Err(_) => not_found(),
    }
}

/// Only plain components survive, which rejects `..`, absolute paths and
/// Windows drive prefixes.
fn is_safe(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, Component::Normal(_)))
}

fn content_type_for(path: &Path) -> &'static str {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return "application/octet-stream";
    };
    match extension.to_ascii_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "txt" => "text/plain; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        "xml" => "application/xml",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::{content_type_for, static_files};
    use crate::{Method, Request, StatusCode};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A unique directory under the OS temp dir that removes itself on drop, so
    /// tests never touch real user data.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!(
                "vr-web-static-{label}-{}-{nanos}-{unique}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn get(handler: &impl crate::Handler, path: &str) -> crate::Response {
        handler.handle(&Request::new(Method::Get, path))
    }

    #[test]
    fn serves_a_file_with_an_extension_content_type() {
        let dir = TempDir::new("serve");
        std::fs::write(dir.path().join("index.html"), b"<h1>hi</h1>").expect("write");
        let handler = static_files(dir.path(), "/static");

        let response = get(&handler, "/static/index.html");
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.body, b"<h1>hi</h1>");
        assert_eq!(
            response.headers.get("content-type"),
            Some("text/html; charset=utf-8")
        );
    }

    #[test]
    fn serves_index_for_the_prefix_and_404s_when_missing() {
        let dir = TempDir::new("index");
        std::fs::write(dir.path().join("index.html"), b"root").expect("write");
        let handler = static_files(dir.path(), "/static");

        assert_eq!(get(&handler, "/static").body, b"root");
        assert_eq!(
            get(&handler, "/static/nope.txt").status,
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn rejects_path_traversal() {
        let dir = TempDir::new("traversal");
        std::fs::write(dir.path().join("index.html"), b"root").expect("write");
        let handler = static_files(dir.path(), "/static");

        assert_eq!(
            get(&handler, "/static/../Cargo.toml").status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            get(&handler, "/static/..%2f..%2fsecret").status,
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn maps_extensions_case_insensitively() {
        assert_eq!(content_type_for(Path::new("a.PNG")), "image/png");
        assert_eq!(
            content_type_for(Path::new("a.xyz")),
            "application/octet-stream"
        );
    }
}
