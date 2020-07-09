use std::collections::HashMap;
use std::str::FromStr;

/// A single result row of a query.
#[derive(Debug, PartialEq)]
pub struct Row {
    value: HashMap<String, Option<String>>,
}

impl Row {
    #[inline]
    pub(crate) fn new() -> Self {
        Self { value: HashMap::new() }
    }

    #[inline]
    pub(crate) fn insert(&mut self, key: String, value: Option<String>) {
        self.value.insert(key, value);
    }

    /// Get the value of a column of the result row.
    #[inline]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.value.get(key)?.as_deref()
    }

    /// Transforms and gets the columns of the result row.
    #[inline]
    pub fn get_into<T: FromStr>(&self, key: &str) -> Result<T,  <T as std::str::FromStr>::Err> {
        T::from_str(self.value.get(key).unwrap_or(&None).as_deref().unwrap_or(""))
    }

    /// Return the number of columns.
    #[inline]
    pub fn column_count(&self) -> usize {
        self.value.len()
    }

    /// Get all the column names.  
    /// Column order is not guaranteed.
    #[inline]
    pub fn column_names(&self) -> Vec<&str> {
        self.value.keys().map(|k| (*k).as_str()).collect::<Vec<_>>()
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
        row.insert("key3".to_string(), Some("42".to_string()));
        assert_eq!(row.get("key1"), Some("value"));
        assert_eq!(row.get("key1").unwrap(), "value");
        assert_eq!(row.get("key2"), None);
        assert_eq!(row.get("key3"), Some("42"));
        assert_eq!(row.column_count(), 3);
        assert_eq!(row.get_into::<String>("key1"), Ok(String::from("value")));
        assert_eq!(row.get_into::<i32>("key3"), Ok(42));
        assert_eq!(row.get_into::<usize>("key3"), Ok(42));
        assert_eq!(row.get_into("key3"), Ok(42));
        assert_eq!(row.get_into("key2"), Ok(String::new()));
        assert_eq!(row.get_into("key1"), Ok(String::from("value")));
        assert!(row.get_into::<u32>("key1").is_err());
        assert!(row.get_into::<u32>("key2").is_err());
        assert!(row.column_names().contains(&"key1"));
        assert!(row.column_names().contains(&"key2"));
        assert!(row.column_names().contains(&"key3"));
        assert!(!row.column_names().contains(&"key4"));
    }
}

