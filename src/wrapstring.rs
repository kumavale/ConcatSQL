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

    /// Return the actual SQL statements that will be executed in the database.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::prelude::*;
    /// assert_eq!(prep!("SELECT").actual_sql(),       "SELECT");
    /// assert_eq!(prep!("O''Reilly").actual_sql(),    "O''Reilly");
    /// assert_eq!(prep!("\"O'Reilly\"").actual_sql(), "\"O'Reilly\"");
    /// assert_eq!((prep!("foo")+"bar").actual_sql(),  "foo'bar'");
    /// assert_eq!((prep!("foo")+42).actual_sql(),     "foo42");
    /// assert_eq!((prep!("foo")+"42").actual_sql(),   "foo'42'");
    /// assert_eq!((prep!()+"O'Reilly").actual_sql(),  "'O''Reilly'");
    /// ```
    pub fn actual_sql(&self) -> &str {
        &self.query
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

// TODO:
//     - sqlite: sqlite3_bind_blob()
//     - mysql:
//     - ~~postgres~~
impl Add<Vec<u8>> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(self, other: Vec<u8>) -> WrapString {
        WrapString { query: self.query + &crate::parser::to_binary_literal(&other) }
    }
}

impl Add<&Vec<u8>> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(self, other: &Vec<u8>) -> WrapString {
        WrapString { query: self.query + &crate::parser::to_binary_literal(other) }
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
        let sql = prep!() + vec![b'A',b'B',b'C'] + &vec![0,1,2];
        if cfg!(feature = "sqlite") || cfg!(feature = "mysql") {
            assert_eq!(sql.actual_sql(), "X'414243'X'000102'");
        } else {
            assert_eq!(sql.actual_sql(), "'\\x414243''\\x000102'");
        }
    }

    #[test]
    fn to_wrapstring() {
        assert_eq!("A".to_wrapstring().actual_sql(), "'A'");
        assert_eq!('A'.to_wrapstring().actual_sql(), "'A'");
        assert_eq!("ABC".to_wrapstring().actual_sql(), "'ABC'");
        assert_eq!(42.to_wrapstring().actual_sql(), "'42'");
        assert_eq!("O'Reilly".to_wrapstring().actual_sql(), "'O''Reilly'");
    }

    mod actual_sql {
        use crate as concatsql;
        use concatsql::prelude::*;

        #[test]
        fn double_quotaion_inside_double_quote() {
            assert_eq!(
                (prep!() + r#"".ow(""inside str"") -> String""#).actual_sql(),
                r#"'".ow(""inside str"") -> String"'"#
            );
            assert_eq!(
                (prep!() + r#"".ow("inside str") -> String""#).actual_sql(),
                r#"'".ow("inside str") -> String"'"#
            );
        }

        #[test]
        fn double_quotaion_inside_sigle_quote() {
            assert_eq!(
                (prep!() + r#""I'm Alice""#).actual_sql(),
                r#"'"I''m Alice"'"#
            );
            assert_eq!(
                (prep!() + r#""I''m Alice""#).actual_sql(),
                r#"'"I''''m Alice"'"#
            );
        }

        #[test]
        fn single_quotaion_inside_double_quote() {
            assert_eq!(
                (prep!() + r#"'.ow("inside str") -> String'"#).actual_sql(),
                r#"'''.ow("inside str") -> String'''"#
            );
        }

        #[test]
        fn single_quotaion_inside_sigle_quote() {
            assert_eq!(
                (prep!() + "'I''m Alice'").actual_sql(),
                "'''I''''m Alice'''"
            );
        }

        #[test]
        fn non_quotaion_inside_sigle_quote() {
            assert_eq!(
                (prep!() + "foo'bar'foo").actual_sql(),
                "'foo''bar''foo'"
            );
        }

        #[test]
        fn non_quotaion_inside_double_quote() {
            assert_eq!(
                (prep!() + r#"foo"bar"foo"#).actual_sql(),
                r#"'foo"bar"foo'"#
            );
        }

        #[test]
        fn empty_string() {
            assert_eq!(prep!("").actual_sql(), "");
        }
    }
}
