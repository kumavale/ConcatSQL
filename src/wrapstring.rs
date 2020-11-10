use std::ops::Add;

#[derive(Clone, Debug, PartialEq)]
pub struct WrapString {
    pub(crate) query: String,
}

impl WrapString {
    pub fn init(s: &'static str) -> Self {
        Self {
            query: s.to_string(),
        }
    }

    pub fn int<T: Clone + ToString>(value: T) -> Result<Self, &'static str> {
        let value = value.to_string();
        if value.parse::<i64>().is_ok() {
            Ok(WrapString::new(&value))
        } else {
            Err("not integer")
        }
    }

    pub(crate) fn new<T: ?Sized + ToString>(s: &T) -> Self {
        Self {
            query: s.to_string(),
        }
    }
}

impl Add for WrapString {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            query: self.query + &other.query,
        }
    }
}

impl<'a> Add<&'a WrapString> for WrapString {
    type Output = WrapString;

    #[inline]
    fn add(self, other: &'a WrapString) -> WrapString {
        WrapString {
            query: self.query + &other.query,
        }
    }
}

impl<T: Sized + ToString> Add<T> for WrapString {
    type Output = WrapString;

    #[inline]
    fn add(self, other: T) -> WrapString {
        WrapString {
            query: self.query + &crate::parser::escape_string(&other.to_string()),
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
    /// # use concatsql::{prep, Wrap};
    /// # let conn = concatsql::sqlite::open(":memory:").unwrap();
    /// assert_eq!(prep!("SELECT").actual_sql(), "SELECT");
    /// assert_eq!("SELECT".actual_sql(),           "'SELECT'");
    /// assert_eq!("O'Reilly".actual_sql(),         "'O''Reilly'");
    /// //prep!("O'Reilly").actual_sql();  // panic
    /// ```
    #[inline]
    fn actual_sql(&self) -> String {
        self.query.clone()
    }
}

impl<T: ?Sized + ToString + std::fmt::Display> Wrap for T {
    fn to_wrapstring(&self) -> WrapString {
        WrapString::new(&crate::parser::escape_string(&self.to_string()))
    }

    fn actual_sql(&self) -> String {
        let s = &self.to_string();
        if s.is_empty() {
            String::new()
        } else {
            crate::parser::escape_string(&s)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate as concatsql;
    use concatsql::prelude::*;

    #[test]
    fn concat_anything_type() {
        let sql = prep!("A") + prep!("B") + "C" + String::from("D") + &String::from("E") + &prep!("F") + 42;
        assert_eq!(sql.actual_sql(), "AB'C''D''E'F'42'");
    }

    #[test]
    fn to_wrapstring() {
        assert_eq!("A".to_wrapstring().actual_sql(), "'A'");
        assert_eq!('A'.to_wrapstring().actual_sql(), "'A'");
        assert_eq!("ABC".to_wrapstring().actual_sql(), "'ABC'");
        assert_eq!(42.to_wrapstring().actual_sql(), "'42'");
        assert_eq!("O'Reilly".to_wrapstring().actual_sql(), "'O''Reilly'");
    }
}
