//! An order-preserving, case-insensitive header collection.

/// Header names are case-insensitive on the wire, so lookups and inserts
/// compare ASCII-case-insensitively while the original spelling is preserved for
/// the response.
#[derive(Debug, Clone, Default)]
pub struct Headers {
    entries: Vec<(String, String)>,
}

impl Headers {
    /// An empty header set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a header, replacing an existing case-insensitive match. Keeping a
    /// single entry per name avoids emitting duplicates like two `content-length`
    /// values, which a client would have to reject.
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        let value = value.into();
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|(existing, _)| existing.eq_ignore_ascii_case(&name))
        {
            entry.1 = value;
        } else {
            self.entries.push((name, value));
        }
    }

    /// Looks up a header by name, ignoring case.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(existing, _)| existing.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Iterates over `(name, value)` in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    /// The number of stored headers.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no headers are stored.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::Headers;

    #[test]
    fn lookup_ignores_case() {
        let mut headers = Headers::new();
        headers.insert("Content-Type", "text/plain");
        assert_eq!(headers.get("content-type"), Some("text/plain"));
    }

    #[test]
    fn insert_replaces_a_differently_cased_name() {
        let mut headers = Headers::new();
        headers.insert("Accept", "text/plain");
        headers.insert("accept", "application/json");
        assert_eq!(headers.len(), 1);
        assert_eq!(headers.get("ACCEPT"), Some("application/json"));
    }
}
