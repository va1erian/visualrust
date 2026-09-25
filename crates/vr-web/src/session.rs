//! An in-memory session store keyed by a session cookie.
//!
//! Sessions live only in this process's memory: they are lost on restart and are
//! not shared between processes. Persistence is deliberately out of scope for
//! this slice — a later store can implement the same surface over a database.
//!
//! Expiry is lazy: a session past its deadline is removed the next time it is
//! touched, so an idle store does not need a background sweeper.

use std::collections::BTreeMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::cookie::{Cookie, CookieError, SameSite};
use crate::request::Request;

/// The cookie a session id is carried in.
pub const SESSION_COOKIE: &str = "vr_session";

/// The default lifetime of a session.
const DEFAULT_TTL: Duration = Duration::from_secs(3600);

/// One live session.
#[derive(Debug, Clone)]
pub struct Session {
    id: String,
    values: BTreeMap<String, String>,
    expires_at: Instant,
}

impl Session {
    /// The session id, as stored in the session cookie.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// A stored value, or `None`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// Sets a value.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    /// Removes a value.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.values.remove(key)
    }

    /// All values, in key order.
    pub fn values(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    /// When the session stops being valid.
    pub fn expires_at(&self) -> Instant {
        self.expires_at
    }

    /// Whether the session is past its deadline at `now`.
    pub fn is_expired(&self, now: Instant) -> bool {
        self.expires_at <= now
    }
}

/// A clonable, thread-safe store. Clones share one map.
#[derive(Clone)]
pub struct SessionStore {
    sessions: Arc<Mutex<BTreeMap<String, Session>>>,
    ttl: Duration,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::with_ttl(DEFAULT_TTL)
    }
}

impl SessionStore {
    /// A store with the default one-hour TTL.
    pub fn new() -> Self {
        Self::default()
    }

    /// A store whose sessions live for `ttl`.
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(BTreeMap::new())),
            ttl,
        }
    }

    /// Creates a fresh session and returns it.
    pub fn create(&self) -> Session {
        let session = Session {
            id: new_id(),
            values: BTreeMap::new(),
            expires_at: Instant::now() + self.ttl,
        };
        self.lock().insert(session.id.clone(), session.clone());
        session
    }

    /// Looks up a live session, evicting it if expired.
    pub fn get(&self, id: &str) -> Option<Session> {
        let now = Instant::now();
        let mut guard = self.lock();
        if guard.get(id).is_some_and(|session| session.is_expired(now)) {
            guard.remove(id);
            return None;
        }
        guard.get(id).cloned()
    }

    /// Returns the session for `id`, or a new one when it is unknown or expired.
    pub fn get_or_create(&self, id: Option<&str>) -> Session {
        if let Some(id) = id
            && let Some(session) = self.get(id)
        {
            return session;
        }
        self.create()
    }

    /// Returns the session carried by `request`, creating one when absent.
    pub fn resolve(&self, request: &Request) -> Session {
        self.get_or_create(request.cookie(SESSION_COOKIE).as_deref())
    }

    /// Sets a value on an existing session. Returns `false` when the session is
    /// unknown or expired, so a caller can regenerate it.
    pub fn set(&self, id: &str, key: impl Into<String>, value: impl Into<String>) -> bool {
        let now = Instant::now();
        let mut guard = self.lock();
        match guard.get_mut(id) {
            Some(session) if !session.is_expired(now) => {
                session.set(key, value);
                true
            }
            _ => false,
        }
    }

    /// Removes a session.
    pub fn remove(&self, id: &str) -> bool {
        self.lock().remove(id).is_some()
    }

    /// Drops every expired session and returns how many were removed.
    pub fn purge_expired(&self) -> usize {
        let now = Instant::now();
        let mut guard = self.lock();
        let before = guard.len();
        guard.retain(|_, session| !session.is_expired(now));
        before - guard.len()
    }

    /// The number of stored sessions, including any not-yet-purged expired ones.
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    /// Builds the cookie a client should send back for `id`.
    pub fn cookie(&self, id: &str) -> Result<Cookie, CookieError> {
        Ok(Cookie::new(SESSION_COOKIE, id)?
            .path("/")
            .http_only()
            .same_site(SameSite::Lax))
    }

    /// A poisoned lock only loses the mutex flag, not the map, so recovering is
    /// safe and keeps one panicking handler from bricking the whole store.
    fn lock(&self) -> MutexGuard<'_, BTreeMap<String, Session>> {
        self.sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// A 128-bit, per-process-random id. `RandomState` is seeded from OS entropy, so
/// ids are not predictable from the clock alone the way a bare counter would be.
fn new_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    let mut first = RandomState::new().build_hasher();
    first.write_u64(counter);
    first.write_u128(nanos);
    let mut second = RandomState::new().build_hasher();
    second.write_u64(counter ^ 0x9e37_79b9_7f4a_7c15);
    second.write_u128(nanos.reverse_bits());
    format!("{:016x}{:016x}", first.finish(), second.finish())
}

#[cfg(test)]
mod tests {
    use super::{SESSION_COOKIE, SessionStore};
    use crate::{Method, Request};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn stores_and_reads_values_by_id() {
        let store = SessionStore::new();
        let session = store.create();
        let id = session.id().to_owned();
        assert!(store.set(&id, "user", "ada"));

        let loaded = store.get(&id).expect("session");
        assert_eq!(loaded.get("user"), Some("ada"));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn expires_sessions_after_the_ttl() {
        let store = SessionStore::with_ttl(Duration::from_millis(20));
        let id = store.create().id().to_owned();
        thread::sleep(Duration::from_millis(40));
        assert!(store.get(&id).is_none());
        assert_eq!(store.purge_expired(), 0);
    }

    #[test]
    fn resolves_a_session_from_the_cookie() {
        let store = SessionStore::new();
        let id = store.create().id().to_owned();
        assert!(store.set(&id, "k", "v"));
        let mut request = Request::new(Method::Get, "/");
        request
            .headers
            .insert("Cookie", format!("{SESSION_COOKIE}={id}"));
        let session = store.resolve(&request);
        assert_eq!(session.id(), id);
        assert_eq!(session.get("k"), Some("v"));
    }

    #[test]
    fn creates_a_session_when_the_cookie_is_missing() {
        let store = SessionStore::new();
        let session = store.resolve(&Request::new(Method::Get, "/"));
        assert_eq!(store.len(), 1);
        assert!(store.cookie(session.id()).is_ok());
    }
}
