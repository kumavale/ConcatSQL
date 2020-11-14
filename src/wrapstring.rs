use std::ops::Add;

/// Wraps a [String](https://doc.rust-lang.org/std/string/struct.String.html) type.
#[derive(Clone, Debug, PartialEq)]
pub struct WrapString {
    pub(crate) query: String,
}

impl WrapString {
    #[doc(hidden)]
    pub fn init(s: &'static str) -> Self {
        Self {
            query: s.to_string(),
        }
    }

    pub(crate) fn new<T: ?Sized + ToString>(s: &T) -> Self {
        Self {
            query: s.to_string(),
        }
    }
}

impl Add for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(self, other: WrapString) -> WrapString {
        WrapString { query: self.query + &other.query }
    }
}

impl<'a> Add<&'a WrapString> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(self, other: &'a WrapString) -> WrapString {
        WrapString { query: self.query + &other.query }
    }
}

impl Add<String> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(self, other: String) -> WrapString {
        WrapString { query: self.query + &crate::parser::escape_string(&other) }
    }
}

impl Add<&String> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(self, other: &String) -> WrapString {
        WrapString { query: self.query + &crate::parser::escape_string(other) }
    }
}

impl Add<&str> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(self, other: &str) -> WrapString {
        WrapString { query: self.query + &crate::parser::escape_string(other) }
    }
}

impl Add<&&str> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(self, other: &&str) -> WrapString {
        WrapString { query: self.query + &crate::parser::escape_string(other) }
    }
}

impl<T: self::Num> Add<T> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(self, other: T) -> WrapString {
        WrapString { query: self.query + &other.to_string() }
    }
}

/// Defines a numeric type that can be concatinated with [WrapString](./struct.WrapString.html).
pub trait Num: ToString {}
macro_rules! impl_Num { ($($type:ty), *) => ($(impl Num for $type {})*) }
impl_Num!(usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64);

/// A trait for converting a value to a [WrapString](./struct.WrapString.html).
pub trait ToWrapString {
    /// Converts the given value to a [WrapString](./struct.WrapString.html).
    fn to_wrapstring(&self) -> WrapString;
}

impl ToWrapString for WrapString {
    fn to_wrapstring(&self) -> WrapString {
        WrapString::new(&self.query)
    }
}

impl<T: ?Sized + ToString + std::fmt::Display> ToWrapString for T {
    fn to_wrapstring(&self) -> WrapString {
        WrapString::new(&crate::parser::escape_string(&self.to_string()))
    }
}

/// A trait for displaying SQL statements that will be executed in the database.
pub trait ActualSQL {
    /// Return the actual SQL statement.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::prelude::*;
    /// assert_eq!(prep!("SELECT").actual_sql(),       "SELECT");
    /// assert_eq!("SELECT".actual_sql(),              "SELECT");
    /// assert_eq!("O'Reilly".actual_sql(),            "O'Reilly");
    /// assert_eq!(prep!("O''Reilly").actual_sql(),    "O''Reilly");
    /// assert_eq!(prep!("\"O'Reilly\"").actual_sql(), "\"O'Reilly\"");
    /// assert_eq!((prep!("foo")+"bar").actual_sql(),  "foo'bar'");
    /// assert_eq!((prep!("foo")+42).actual_sql(),     "foo42");
    /// assert_eq!((prep!("foo")+"42").actual_sql(),   "foo'42'");
    /// ```
    fn actual_sql(&self) -> &str;
}

impl ActualSQL for WrapString {
    fn actual_sql(&self) -> &str {
        &self.query
    }
}

impl ActualSQL for &'static str {
    fn actual_sql(&self) -> &str {
        self
    }
}

#[cfg(test)]
mod tests {
    use crate as concatsql;
    use concatsql::prelude::*;

    #[test]
    #[allow(clippy::op_ref, clippy::deref_addrof, clippy::identity_op, clippy::approx_constant)]
    fn concat_anything_type() {
        let sql = prep!("A") + prep!("B") + "C" + String::from("D") + &String::from("E") + &prep!("F") + 42 + 3.14;
        assert_eq!(sql.actual_sql(), "AB'C''D''E'F423.14");
        let sql = prep!() + String::from("A") + &String::from("B") + *&&String::from("C") + **&&&String::from("D");
        assert_eq!(sql.actual_sql(), "'A''B''C''D'");
        let sql = prep!() + "A" + &"B" + *&&"C" + **&&&"D";
        assert_eq!(sql.actual_sql(), "'A''B''C''D'");
        let sql = prep!() + 0usize + 1u8 + 2u16 + 3u32 + 4u64 + 5u128 + 6isize + 7i8 + 8i16 + 9i32 + 0i64 + 1i128 + 2f32 + 3f64;
        assert_eq!(sql.actual_sql(), "01234567890123");
        let sql = prep!() + f32::MAX + f32::INFINITY + f32::NAN;
        assert_eq!(sql.actual_sql(), "340282350000000000000000000000000000000infNaN");
    }

    #[test]
    fn to_wrapstring() {
        assert_eq!("A".to_wrapstring().actual_sql(), "'A'");
        assert_eq!('A'.to_wrapstring().actual_sql(), "'A'");
        assert_eq!("ABC".to_wrapstring().actual_sql(), "'ABC'");
        assert_eq!(42.to_wrapstring().actual_sql(), "'42'");
        assert_eq!("O'Reilly".to_wrapstring().actual_sql(), "'O''Reilly'");
    }

    #[test]
    fn actual_sql() {
        assert_eq!("foo".actual_sql(), "foo");
        assert_eq!(prep!("bar").actual_sql(), "bar");
        assert_eq!((prep!("bar") + "baz").actual_sql(), "bar'baz'");
    }
}
