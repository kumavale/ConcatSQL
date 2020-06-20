//! TODO

use std::collections::HashMap;

/// TODO
pub struct Row {
    value: HashMap<String, Option<String>>,
}

impl Row {
    pub(crate) fn new() -> Self {
        Self { value: HashMap::new() }
    }

    pub(crate) fn insert(&mut self, key: String, value: Option<String>) {
        self.value.insert(key, value);
    }

    /// TODO
    pub fn get(&self, key: &str) -> Option<String> {
        self.value.get(key)?.as_ref().map(|v| v.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row() {
        let mut row = Row::new();
        row.insert("key1".to_string(), Some("value".to_string()));
        row.insert("key2".to_string(), None);
        assert_eq!(row.get("key1"), Some("value".to_string()));
        assert_eq!(row.get("key1").unwrap(), "value");
        assert_eq!(row.get("key2"), None);
        assert_eq!(row.get("key3"), None);
    }
}

