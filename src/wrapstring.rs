use std::ops::Add;

/// TODO
#[derive(Clone, Debug)]
pub enum Param<'a> {
    I32(i32),
    I64(i64),
    F64(f64),
    Blob(&'a [u8]),
    Text(String),
    Null,
}

/// Wraps a [String](https://doc.rust-lang.org/std/string/struct.String.html) type.
#[derive(Clone, Debug)]
pub struct WrapString<'a> {
    pub(crate) prepare: String,
    pub(crate) params:  Vec<Param<'a>>,
}

impl<'a> WrapString<'a> {
    #[doc(hidden)]
    pub fn init(s: &'static str) -> Self {
        Self {
            prepare: s.to_string(),
            params:  Vec::new(),
        }
    }

    pub(crate) fn new<T: ?Sized + ToString>(s: &T) -> Self {
        Self {
            prepare: s.to_string(),
            params:  Vec::new(),
        }
    }
}

impl<'a> Add for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: WrapString<'a>) -> WrapString<'a> {
        self.prepare.push_str(&other.prepare);
        self.params.extend(other.params);
        self
    }
}

impl<'a, 'b> Add<&'b WrapString<'a>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'b WrapString<'a>) -> WrapString<'a> {
        self.prepare.push_str(&other.prepare);
        self.params.extend_from_slice(&other.params);
        self
    }
}

impl<'a> Add<String> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: String) -> WrapString<'a> {
        self.prepare.push('?');
        self.params.push(Param::Text(other));
        self
    }
}

impl<'a, 'b> Add<&'b String> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'b String) -> WrapString<'a> {
        //WrapString { query: self.query + &crate::parser::escape_string(other) }
        self.prepare.push('?');
        self.params.push(Param::Text(other.to_string()));
        self
    }
}

impl<'a> Add<&'a str> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'a str) -> WrapString<'a> {
        //WrapString { query: self.query + &crate::parser::escape_string(other) }
        self.prepare.push('?');
        self.params.push(Param::Text(other.to_string()));
        self
    }
}

impl<'a> Add<&&'a str> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &&'a str) -> WrapString<'a> {
        //WrapString { query: self.query + &crate::parser::escape_string(other) }
        self.prepare.push('?');
        self.params.push(Param::Text(other.to_string()));
        self
    }
}

impl<'a, T: self::Num> Add<T> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: T) -> WrapString<'a> {
        //WrapString { query: self.query + &other.to_string() }
        // TODO
        self.prepare.push('?');
        self.params.push(Param::Text("TODO".to_string()));
        self
    }
}

/// Defines a numeric type that can be concatinated with [WrapString](./struct.WrapString.html).
pub trait Num: ToString {}
macro_rules! impl_Num { ($($type:ty), *) => ($(impl Num for $type {})*) }
impl_Num!(usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64);

/// A trait for converting a value to a [WrapString](./struct.WrapString.html).
pub trait ToWrapString<'a> {
    /// Converts the given value to a [WrapString](./struct.WrapString.html).
    fn to_wrapstring(&self) -> WrapString<'a>;
}

impl<'a> ToWrapString<'a> for WrapString<'a> {
    fn to_wrapstring(&self) -> WrapString<'a> {
        self.clone()
    }
}

impl<'a, T: ?Sized + ToString + std::fmt::Display> ToWrapString<'a> for T {
    fn to_wrapstring(&self) -> WrapString<'a> {
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

impl<'a> ActualSQL for WrapString<'a> {
    fn actual_sql(&self) -> String {
        // TODO
        self.prepare.to_string()
    }
}

impl ActualSQL for &'static str {
    fn actual_sql(&self) -> String {
        self.to_string()
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
