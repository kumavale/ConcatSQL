use std::collections::HashMap;

/// A single result row of a query.
#[derive(Debug, PartialEq)]
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

    /// Get the value of a column of the result row.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.value.get(key)?.as_deref()
    }

    /// Return the number of columns.
    pub fn column_count(&self) -> usize {
        self.value.len()
    }

    /// Get all the column names.  
    /// Column order is not guaranteed.
    pub fn column_names(&self) -> Vec<&str> {
        self.value.keys().map(|k| (*k).as_str()).collect::<Vec<_>>()
    }

    /// Return the name of the column.  
    /// Column order is not guaranteed.
    pub fn column_name(&self, col: usize) -> Option<&str> {
        self.column_names().get(col).copied()
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
        assert_eq!(row.get("key1"), Some("value"));
        assert_eq!(row.get("key1").unwrap(), "value");
        assert_eq!(row.get("key2"), None);
        assert_eq!(row.get("key3"), None);
        assert_eq!(row.column_count(), 2);
        assert!(row.column_names() == vec!["key1", "key2"] || row.column_names() == vec!["key2", "key1"]);
        assert!(row.column_name(0) == Some("key1") || row.column_name(0) == Some("key2"));
        assert!(row.column_name(1) == Some("key1") || row.column_name(1) == Some("key2"));
        assert_eq!(row.column_name(2), None);
    }
}

