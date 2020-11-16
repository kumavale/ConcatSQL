use std::str::FromStr;
use indexmap::map::IndexMap;
use crate::error::Error;

type IndexMapPairs = IndexMap<String, Option<String>>;

/// A single result row of a query.
#[derive(Debug, PartialEq)]
pub struct Row {
    pairs: IndexMapPairs,
}

impl Row {
    #[inline]
    pub(crate) fn new() -> Self {
        Self { pairs: IndexMap::new() }
    }

    #[inline]
    pub(crate) fn insert(&mut self, key: String, value: Option<String>) {
        self.pairs.insert(key, value);
    }

    /// Get the value of a column of the result row.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::prelude::*;
    /// # let conn = concatsql::sqlite::open(":memory:").unwrap();
    /// for row in conn.rows("SELECT 1").unwrap() {
    ///     assert_eq!(row.get(0).unwrap(),   "1");
    ///     assert_eq!(row.get("1").unwrap(), "1");
    /// }
    /// ```
    pub fn get<T: Get>(&self, key: T) -> Option<&str> {
        key.get(&self.pairs)
    }

    /// Transforms and gets the columns of the result row.  
    /// &#x26a0;&#xfe0f; If column is not found then execute `U::from_str("")`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::prelude::*;
    /// # let conn = concatsql::sqlite::open(":memory:").unwrap();
    /// for row in conn.rows("SELECT 1").unwrap() {
    ///     assert_eq!(row.get_into::<_, i32>(0).unwrap(),   1);
    ///     assert_eq!(row.get_into::<_, i32>("1").unwrap(), 1);
    ///
    ///     assert_eq!(row.get_into::<_, String>(0).unwrap(),   "1");
    ///     assert_eq!(row.get_into::<_, String>("1").unwrap(), "1");
    ///
    ///     let one: u8 = row.get_into(0).unwrap();
    ///     assert_eq!(one, 1u8);
    /// }
    /// ```
    #[inline]
    pub fn get_into<T: Get, U: FromSql>(&self, key: T) -> Result<U, Error> {
        key.get_into::<U>(&self.pairs)
    }

    /// Return the number of columns.
    #[inline]
    pub fn column_count(&self) -> usize {
        self.pairs.len()
    }

    /// Determines if there are any values in the row.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pairs.len() == 0
    }

    /// Get all the column names.  
    /// Column order is not guaranteed.
    #[inline]
    pub fn column_names(&self) -> Vec<&str> {
        self.pairs.keys().map(|k| (*k).as_str()).collect::<Vec<_>>()
    }
}

/// A trait implemented by types that can index into columns of a row.
pub trait Get {
    fn get<'a>(&self, pairs: &'a IndexMapPairs) -> Option<&'a str>;
    fn get_into<'a, U: FromSql>(&self, pairs: &'a IndexMapPairs) -> Result<U, Error>;
}

impl Get for str {
    fn get<'a>(&self, pairs: &'a IndexMapPairs) -> Option<&'a str> {
        pairs.get(self)?.as_deref()
    }

    fn get_into<'a, U: FromSql>(&self, pairs: &'a IndexMapPairs) -> Result<U, Error> {
        U::from_sql(pairs.get(self).unwrap_or(&None).as_deref().unwrap_or(""))
    }
}

impl Get for usize {
    fn get<'a>(&self, pairs: &'a IndexMapPairs) -> Option<&'a str> {
        pairs.get_index(*self)?.1.as_deref()
    }

    fn get_into<'a, U: FromSql>(&self, pairs: &'a IndexMapPairs) -> Result<U, Error> {
        U::from_sql(pairs.get_index(*self).unwrap_or((&String::new(), &None)).1.as_deref().unwrap_or(""))
    }
}

impl<'b, T> Get for &'b T where T: Get + ?Sized {
    fn get<'a>(&self, pairs: &'a IndexMapPairs) -> Option<&'a str> {
        T::get(self, &pairs)
    }

    fn get_into<'a, U: FromSql>(&self, pairs: &'a IndexMapPairs) -> Result<U, Error> {
        T::get_into(self, &pairs)
    }
}

/// Parse a value from a sql string.
pub trait FromSql: Sized {
    fn from_sql(s: &str) -> Result<Self, Error>;
}

macro_rules! from_sql_impl {
    ( $($t:ty),* ) => {$(
        impl FromSql for $t {
            #[doc(hidden)]
            fn from_sql(s: &str) -> Result<Self, Error> {
                Self::from_str(s).map_err(|_|Error::ParseError)
            }
        }
    )*};
    ( $($t:ty,)* ) => { from_sql_impl! { $( $t ),* } };
}
from_sql_impl! {
    std::net::IpAddr,
    std::net::SocketAddr,
    bool,
    char,
    f32, f64,
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
    std::ffi::OsString,
    std::net::Ipv4Addr,
    std::net::Ipv6Addr,
    std::net::SocketAddrV4,
    std::net::SocketAddrV6,
    std::num::NonZeroI8,
    std::num::NonZeroI16,
    std::num::NonZeroI32,
    std::num::NonZeroI64,
    std::num::NonZeroI128,
    std::num::NonZeroIsize,
    std::num::NonZeroU8,
    std::num::NonZeroU16,
    std::num::NonZeroU32,
    std::num::NonZeroU64,
    std::num::NonZeroU128,
    std::num::NonZeroUsize,
    std::path::PathBuf,
    String,
}

impl FromSql for Vec<u8> {
    #[doc(hidden)]
    fn from_sql(s: &str) -> Result<Self, Error> {
        Ok(
            (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i+2], 16).map_err(|_|()))
            .collect::<Result<Vec<u8>, ()>>().map_err(|_|Error::ParseError)?
        )
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
        assert_eq!(row.get("key4"), None);
        assert_eq!(row.get(0), Some("value"));
        assert_eq!(row.get(0).unwrap(), "value");
        assert_eq!(row.get(1), None);
        assert_eq!(row.get(2), Some("42"));
        assert_eq!(row.get(3), None);

        assert_eq!(row.get_into::<&str, String>("key1"), Ok(String::from("value")));
        assert_eq!(row.get_into::<&str, i32>("key3"), Ok(42));
        assert_eq!(row.get_into::<&str, usize>("key3"), Ok(42));
        assert_eq!(row.get_into("key3"), Ok(42));
        assert_eq!(row.get_into("key2"), Ok(String::new()));
        assert_eq!(row.get_into("key1"), Ok(String::from("value")));
        assert_eq!(row.get_into::<usize, String>(0), Ok(String::from("value")));
        assert_eq!(row.get_into::<usize, i32>(2), Ok(42));
        assert_eq!(row.get_into::<usize, usize>(2), Ok(42));
        assert_eq!(row.get_into(2), Ok(42));
        assert_eq!(row.get_into(1), Ok(String::new()));
        assert_eq!(row.get_into(0), Ok(String::from("value")));
        assert!(row.get_into::<&str, u32>("key1").is_err());
        assert!(row.get_into::<&str, u32>("key2").is_err());
        assert!(row.get_into::<&str, u32>("key4").is_err());
        assert!(!row.get_into::<&str, String>("key4").is_err());  // I want to make result to Err
        assert!(row.get_into::<usize, u32>(0).is_err());
        assert!(row.get_into::<usize, u32>(1).is_err());
        assert!(row.get_into::<usize, u32>(99).is_err());
        assert!(!row.get_into::<usize, String>(99).is_err());  // I want to make result to Err

        assert_eq!(row.get_into::<_, String>("key1"), Ok(String::from("value")));
        assert_eq!(row.get_into::<_, i32>("key3"), Ok(42));
        assert_eq!(row.get_into::<_, usize>("key3"), Ok(42));
        assert_eq!(row.get_into("key3"), Ok(42));
        assert_eq!(row.get_into("key2"), Ok(String::new()));
        assert_eq!(row.get_into("key1"), Ok(String::from("value")));
        assert_eq!(row.get_into::<_, String>(0), Ok(String::from("value")));
        assert_eq!(row.get_into::<_, i32>(2), Ok(42));
        assert_eq!(row.get_into::<_, usize>(2), Ok(42));
        assert_eq!(row.get_into(2), Ok(42));
        assert_eq!(row.get_into(1), Ok(String::new()));
        assert_eq!(row.get_into(0), Ok(String::from("value")));
        assert!(row.get_into::<_, u32>("key1").is_err());
        assert!(row.get_into::<_, u32>("key2").is_err());
        assert!(row.get_into::<_, u32>("key4").is_err());
        assert!(!row.get_into::<_, String>("key4").is_err());  // I want to make result to Err
        assert!(row.get_into::<_, u32>(0).is_err());
        assert!(row.get_into::<_, u32>(1).is_err());
        assert!(row.get_into::<_, u32>(99).is_err());
        assert!(!row.get_into::<_, String>(99).is_err());  // I want to make result to Err

        assert_eq!(row.column_count(), 3);

        assert!(row.column_names().contains(&"key1"));
        assert!(row.column_names().contains(&"key2"));
        assert!(row.column_names().contains(&"key3"));
        assert!(!row.column_names().contains(&"key4"));

        assert!(!row.is_empty());

        assert_eq!(row.get(&"key1"), Some("value"));
        assert_eq!(row.get(&&&&&&&&"key1"), Some("value"));
        assert_eq!(row.get(&*String::from("key1")), Some("value"));
        assert_eq!(row.get(&0), Some("value"));

        row.insert("ABC".to_string(), Some("414243".to_string()));
        assert_eq!(row.get_into::<_, Vec<u8>>("ABC"), Ok(vec![b'A',b'B',b'C']));
        assert!(row.get_into::<_, i8>("ABC").is_err());
        assert!(row.get_into::<_, u8>("ABC").is_err());
        assert!(row.get_into::<_, i16>("ABC").is_err());
        assert!(row.get_into::<_, u16>("ABC").is_err());
        assert_eq!(row.get_into::<_, i32>("ABC"),   Ok(414243));
        assert_eq!(row.get_into::<_, u32>("ABC"),   Ok(414243));
        assert_eq!(row.get_into::<_, i64>("ABC"),   Ok(414243));
        assert_eq!(row.get_into::<_, u64>("ABC"),   Ok(414243));
        assert_eq!(row.get_into::<_, i128>("ABC"),  Ok(414243));
        assert_eq!(row.get_into::<_, u128>("ABC"),  Ok(414243));
        assert_eq!(row.get_into::<_, isize>("ABC"), Ok(414243));
        assert_eq!(row.get_into::<_, usize>("ABC"), Ok(414243));
    }
}

