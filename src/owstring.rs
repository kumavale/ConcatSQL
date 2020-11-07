use std::ops::Add;

use crate::parser::escape_string;

#[derive(Clone, Debug, PartialEq)]
pub struct OwString {
    pub(crate) query: String,
}

impl OwString {
    pub fn new<T: ?Sized + std::string::ToString>(s: &T) -> Self {
        Self {
            query: s.to_string(),
        }
    }

    /// Return the actual SQL statement.
    ///
    /// # Examples
    ///
    /// ```
    /// # use exowsql::{prepare, bind};
    /// # let conn = exowsql::sqlite::open(":memory:").unwrap();
    /// assert_eq!(prepare!("SELECT").actual_sql(),   "SELECT");
    /// assert_eq!(bind!("SELECT").actual_sql(),      "'SELECT'");
    /// assert_eq!(bind!("O'Reilly").actual_sql(),    "'O''Reilly'");
    /// //prepare!("O'Reilly").actual_sql();  // panic
    /// ```
    #[inline]
    pub fn actual_sql(&self) -> &str {
        &self.query
    }
}

impl Add for OwString {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            query: self.query + &other.query,
        }
    }
}

impl<'a> Add<&'a OwString> for OwString {
    type Output = OwString;

    fn add(self, other: &'a OwString) -> OwString {
        OwString {
            query: self.query + &other.query.clone(),
        }
    }
}

/// TODO docs
#[macro_export]
macro_rules! prepare {
    ($query:expr) => {
        {
            use std::sync::Once;
            use exowsql::{OwString, check_valid_literal};
            static START: Once = Once::new();
            START.call_once(|| check_valid_literal($query).unwrap());
            OwString::new($query)
        }
    };
}

/// TODO docs
#[macro_export]
macro_rules! bind {
    ($value:expr) => { exowsql::_bind($value) };
}
#[doc(hidden)]
pub fn _bind<T: ToString>(value: T) -> OwString {
    let escaped = escape_string(&value.to_string(), |c| c == '\'');
    OwString::new(&escaped)
}

