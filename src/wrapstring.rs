use std::ops::Add;

#[derive(Clone, Debug, PartialEq)]
pub struct WrapString {
    pub(crate) query: String,
}

impl WrapString {
    pub fn new<T: ?Sized + std::string::ToString>(s: &T) -> Self {
        Self {
            query: s.to_string(),
        }
    }
}

impl Add for WrapString {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            query: self.query + &other.query,
        }
    }
}

impl<'a> Add<&'a WrapString> for WrapString {
    type Output = WrapString;

    fn add(self, other: &'a WrapString) -> WrapString {
        WrapString {
            query: self.query + &other.query.clone(),
        }
    }
}

impl Add<String> for WrapString {
    type Output = WrapString;

    fn add(self, other: String) -> WrapString {
        WrapString {
            query: self.query + &crate::parser::escape_string(&other, |c| c == '\''),
        }
    }
}

impl Add<&str> for WrapString {
    type Output = WrapString;

    fn add(self, other: &str) -> WrapString {
        WrapString {
            query: self.query + &crate::parser::escape_string(other, |c| c == '\''),
        }
    }
}

pub trait Wrap {
    fn to_wrapstring(&self) -> WrapString;
    fn actual_sql(&self) -> String;
}

impl Wrap for WrapString {
    fn to_wrapstring(&self) -> WrapString {
        WrapString::new(&self.query)
    }

    /// Return the actual SQL statement.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::{prepare, Wrap};
    /// # let conn = concatsql::sqlite::open(":memory:").unwrap();
    /// assert_eq!(prepare!("SELECT").actual_sql(), "SELECT");
    /// assert_eq!("SELECT".actual_sql(),           "'SELECT'");
    /// assert_eq!("O'Reilly".actual_sql(),         "'O''Reilly'");
    /// //prepare!("O'Reilly").actual_sql();  // panic
    /// ```
    #[inline]
    fn actual_sql(&self) -> String {
        self.query.clone()
    }
}

impl<T: ?Sized + ToString + std::fmt::Display> Wrap for T {
    fn to_wrapstring(&self) -> WrapString {
        WrapString::new(&crate::parser::escape_string(&self.to_string(), |c| c == '\''))
    }

    fn actual_sql(&self) -> String {
        crate::parser::escape_string(&self.to_string(), |c| c == '\'')
    }
}

