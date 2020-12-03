use std::ops::Add;
use std::borrow::Cow;
use crate::parser::{escape_string, to_binary_literal};
use uuid::Uuid;

/// Values that can be bound as static placeholders.
#[derive(Clone, Debug, PartialEq)]
pub enum Value<'a> {
    Null,
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Text(Cow<'a, str>),
    Bytes(Vec<u8>),
}

/// Wraps a [String](https://doc.rust-lang.org/std/string/struct.String.html) type.
#[derive(Clone, Debug, PartialEq)]
pub struct WrapString<'a> {
    pub(crate) query:  Vec<Option<Cow<'a, str>>>,
    pub(crate) params: Vec<Value<'a>>,
}

impl<'a> WrapString<'a> {
    #[doc(hidden)]
    pub fn init(s: &'static str) -> Self {
        Self {
            query:  vec![ Some(Cow::Borrowed(s)) ],
            params: Vec::new(),
        }
    }

    #[doc(hidden)]
    pub const fn null() -> Self {
        Self {
            query:  Vec::new(),
            params: Vec::new(),
        }
    }

    pub(crate) fn new<T: ?Sized + ToString>(s: &T) -> Self {
        Self {
            query:  vec![ Some(Cow::Owned(s.to_string())) ],
            params: Vec::new(),
        }
    }

    /// Simulates the SQL statement that will be executed in the database.
    ///
    /// If multiple features are specified, they may not be displayed correctly.  
    /// &#x26a0;&#xfe0f; This crate actually using static placeholders.  
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::prelude::*;
    /// assert_eq!(prep!("SELECT").simulate(),       "SELECT");
    /// assert_eq!(prep!("O''Reilly").simulate(),    "O''Reilly");
    /// assert_eq!(prep!("\"O'Reilly\"").simulate(), "\"O'Reilly\"");
    /// assert_eq!((prep!("foo")+"bar").simulate(),  "foo'bar'");
    /// assert_eq!((prep!("foo")+42i32).simulate(),  "foo42");
    /// assert_eq!((prep!("foo")+"42").simulate(),   "foo'42'");
    /// assert_eq!((prep!()+"O'Reilly").simulate(),  "'O''Reilly'");
    /// ```
    pub fn simulate(&self) -> String {
        let mut query = String::new();
        let mut index = 0;
        for part in &self.query {
            match part {
                Some(s) => query.push_str(&s),
                None => {
                    match &self.params[index] {
                        Value::Null         => query.push_str("NULL"),
                        Value::I32(value)   => query.push_str(&value.to_string()),
                        Value::I64(value)   => query.push_str(&value.to_string()),
                        Value::F32(value)   => query.push_str(&value.to_string()),
                        Value::F64(value)   => query.push_str(&value.to_string()),
                        Value::Text(value)  => query.push_str(&escape_string(&value)),
                        Value::Bytes(value) => query.push_str(&to_binary_literal(&value)),
                    }
                    index += 1;
                }
            }
        }
        query
    }
}

impl<'a> Add for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: WrapString<'a>) -> WrapString<'a> {
        self.query .extend_from_slice(&other.query);
        self.params.extend_from_slice(&other.params);
        self
    }
}

impl<'a, 'b> Add<&'b WrapString<'a>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'b WrapString<'a>) -> WrapString<'a> {
        self.query .extend_from_slice(&other.query);
        self.params.extend_from_slice(&other.params);
        self
    }
}

impl<'a> Add<String> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: String) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Text(Cow::Owned(other)));
        self
    }
}

impl<'a> Add<&'a String> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'a String) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Text(Cow::Borrowed(other)));
        self
    }
}

impl<'a> Add<&'a str> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'a str) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Text(Cow::Borrowed(other)));
        self
    }
}

impl<'a> Add<&'a &str> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'a &str) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Text(Cow::Borrowed(other)));
        self
    }
}

impl<'a> Add<std::borrow::Cow<'a, str>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: std::borrow::Cow<'a, str>) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Text(other));
        self
    }
}

impl<'a> Add<&'a std::borrow::Cow<'a, str>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'a std::borrow::Cow<'a, str>) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Text(Cow::Borrowed(&*other)));
        self
    }
}

impl<'a> Add<Vec<u8>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: Vec<u8>) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Bytes(other));
        self
    }
}

impl<'a> Add<&Vec<u8>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &Vec<u8>) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Bytes(other.clone()));
        self
    }
}

macro_rules! impl_add_I32_for_WrapString {
    ( $($t:ty),* ) => ($(
        impl<'a> Add<$t> for WrapString<'a> {
            type Output = WrapString<'a>;
            #[inline]
            fn add(mut self, other: $t) -> WrapString<'a> {
                self.query .push(None);
                self.params.push(Value::I32(other as i32));
                self
            }
        }
    )*)
}

macro_rules! impl_add_I64_for_WrapString {
    ( $($t:ty),* ) => ($(
        impl<'a> Add<$t> for WrapString<'a> {
            type Output = WrapString<'a>;
            #[inline]
            fn add(mut self, other: $t) -> WrapString<'a> {
                self.query .push(None);
                self.params.push(Value::I64(other as i64));
                self
            }
        }
    )*)
}

/// Sent as a 32-byte string.
impl<'a> Add<Uuid> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: Uuid) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Text(Cow::Owned(format!("{:X}", other.to_simple()))));
        self
    }
}

/// Sent as a 32-byte string.
impl<'a> Add<&Uuid> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &Uuid) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Text(Cow::Owned(format!("{:X}", other.to_simple_ref()))));
        self
    }
}

impl_add_I32_for_WrapString!(u8, u16, u32, i8, i16, i32);
impl_add_I64_for_WrapString!(u64, i64);

#[cfg(target_pointer_width = "16")]
#[cfg(target_pointer_width = "32")]
impl_add_I32_for_WrapString!(usize, isize);

#[cfg(target_pointer_width = "64")]
impl_add_I64_for_WrapString!(usize, isize);

impl<'a> Add<f32> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: f32) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::F32(other));
        self
    }
}

impl<'a> Add<f64> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: f64) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::F64(other));
        self
    }
}

macro_rules! impl_add_Option_for_WrapString {
    ( $($t:ty),* ) => {$(
        impl<'a> Add<Option<$t>> for WrapString<'a> {
            type Output = WrapString<'a>;
            #[inline]
            fn add(mut self, other: Option<$t>) -> WrapString<'a> {
                match other {
                    Some(other) => self.add(other),
                    None => {
                        self.query .push(None);
                        self.params.push(Value::Null);
                        self
                    }
                }
            }
        }
    )*};
    ( $($t:ty,)* ) => { impl_add_Option_for_WrapString!{ $( $t ),* } }
}

impl_add_Option_for_WrapString! {
    String,
    &'a String,
    &'a str,
    std::borrow::Cow<'a, str>,
    Vec<u8>,
    &'a Vec<u8>,
    u8, u16, u32, u64, usize,
    i8, i16, i32, i64, isize,
    f32, f64,
    Uuid,
}

impl<'a> Add<()> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, _other: ()) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Null);
        self
    }
}

/// In operator with string arrays.  
/// If the array is empty, it will be ignored.
///
/// # Examples
///
/// ```
/// # use concatsql::prelude::*;
/// let names: Vec<&str> = vec![];
/// assert_eq!((prep!("(")+names+prep!(")")).simulate(), "(NULL)");
/// let names: Vec<&str> = vec!["foo","bar"];
/// assert_eq!((prep!("(")+names+prep!(")")).simulate(), "('foo','bar')");
/// ```
impl<'a> Add<Vec<String>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: Vec<String>) -> WrapString<'a> {
        if other.is_empty() {
            self.query .push(None);
            self.params.push(Value::Null);
            return self;
        }
        if let Some(first) = other.first() {
            self.query.push(None);
            self.params.push(Value::Text(Cow::Owned(first.to_string())));
        }
        for param in other.into_iter().skip(1) {
            self.query.push(Some(Cow::Borrowed(",")));
            self.query.push(None);
            self.params.push(Value::Text(Cow::Owned(param)));
        }
        self
    }
}

macro_rules! impl_add_arrays_borrowed_for_WrapString {
    ( $($t:ty),* ) => {$(
        /// In operator with string arrays.  
        /// If the array is empty, it will be ignored.
        ///
        /// # Examples
        ///
        /// ```
        /// # use concatsql::prelude::*;
        /// let names: Vec<&str> = vec![];
        /// assert_eq!((prep!("(")+names+prep!(")")).simulate(), "(NULL)");
        /// let names: Vec<&str> = vec!["foo","bar"];
        /// assert_eq!((prep!("(")+names+prep!(")")).simulate(), "('foo','bar')");
        /// ```
        impl<'a> Add<$t> for WrapString<'a> {
            type Output = WrapString<'a>;
            #[inline]
            fn add(mut self, other: $t) -> WrapString<'a> {
                if other.is_empty() {
                    self.query .push(None);
                    self.params.push(Value::Null);
                    return self;
                }
                if let Some(first) = other.first() {
                    self.query.push(None);
                    self.params.push(Value::Text(Cow::Borrowed(first)));
                }
                for param in other.iter().skip(1) {
                    self.query.push(Some(Cow::Borrowed(",")));
                    self.query.push(None);
                    self.params.push(Value::Text(Cow::Borrowed(param)));
                }
                self
            }
        }
    )*};
    ( $($t:ty,)* ) => { impl_add_arrays_borrowed_for_WrapString!{ $( $t ),* } }
}

impl_add_arrays_borrowed_for_WrapString!{
    Vec<&'a str>,
    &'a Vec<String>,
    &'a Vec<&'a str>,
    &'a [&'a str],
    &'a [String],
}


/// A trait for converting a value to a [WrapString](./struct.WrapString.html).
pub trait IntoWrapString<'a> {
    /// Converts the given value to a [WrapString](./struct.WrapString.html).
    #[doc(hidden)]
    fn into_wrapstring(self) -> WrapString<'a>;
}

impl<'a> IntoWrapString<'a> for WrapString<'a> {
    #[doc(hidden)]
    fn into_wrapstring(self) -> WrapString<'a> {
        self
    }
}

impl<'a, 'b> IntoWrapString<'a> for &'b WrapString<'a> {
    #[doc(hidden)]
    fn into_wrapstring(self) -> WrapString<'a> {
        self.clone()
    }
}

impl<'a> IntoWrapString<'a> for &'static str {
    #[doc(hidden)]
    fn into_wrapstring(self) -> WrapString<'a> {
        WrapString::init(self)
    }
}


#[cfg(test)]
mod tests {
    use crate as concatsql;
    use concatsql::prelude::*;

    #[test]
    #[allow(
        clippy::op_ref,
        clippy::deref_addrof,
        clippy::identity_op,
        clippy::approx_constant,
        clippy::many_single_char_names,
    )]
    fn concat_anything_type() {
        use std::borrow::Cow;
        let a = String::from("A");
        let b = &String::from("B");
        let c = &**&&String::from("C");
        let d = &***&&&String::from("D");
        let e = String::from("E");
        let sql: WrapString = prep!("A") + prep!("B") + "C" + String::from("D") + &e + &prep!("F") + 42 + 3.14;
        assert_eq!(sql.simulate(), "AB'C''D''E'F423.14");
        let sql = prep!() + a + b + c + d;
        assert_eq!(sql.simulate(), "'A''B''C''D'");
        let sql = prep!() + "A" + &"B" + *&&"C" + **&&&"D";
        assert_eq!(sql.simulate(), "'A''B''C''D'");
        let sql = prep!() + 0usize + 1u8 + 2u16 + 3u32 + 4u64 + 5isize + 6i8 + 7i16 + 8i32 + 9i64 + 0f32 + 1f64;
        assert_eq!(sql.simulate(), "012345678901");
        let sql = prep!() + f32::MAX + f32::INFINITY + f32::NAN;
        assert_eq!(sql.simulate(), "340282350000000000000000000000000000000infNaN");
        let sql = prep!() + vec![b'A',b'B',b'C'] + &vec![0,1,2];
        if cfg!(feature = "sqlite") || cfg!(feature = "mysql") {
            assert_eq!(sql.simulate(), "X'414243'X'000102'");
        } else {
            assert_eq!(sql.simulate(), "'\\x414243''\\x000102'");
        }
        let sql = prep!() + Cow::Borrowed("A") + &Cow::Borrowed("B") + Cow::Owned("C".to_string());
        assert_eq!(sql.simulate(), "'A''B''C'");
        let sql = prep!("A") + Some("B") + Some(String::from("C")) + Some(0i32) + Some(3.14f32) + Some(42i32) + None as Option<i32> + ();
        assert_eq!(sql.simulate(), "A'B''C'03.1442NULLNULL");
        let vec: Vec<String> = Vec::new();
        let sql = prep!("(") + vec + prep!(")");
        assert_eq!(sql.simulate(), "(NULL)");
        let sql = prep!("(") + vec!["A"] + prep!(")");
        assert_eq!(sql.simulate(), "('A')");
        let sql = prep!("(") + vec!["A","B"] + prep!(")");
        assert_eq!(sql.simulate(), "('A','B')");
        let sql = prep!("(") + vec![String::from("A"),String::from("B")] + prep!(")");
        assert_eq!(sql.simulate(), "('A','B')");
        let vec = vec!["A","B"];
        let sql = prep!("(") + &vec + prep!(")");
        assert_eq!(sql.simulate(), "('A','B')");
        let vec = vec![String::from("A"),String::from("B")];
        let sql = prep!("(") + &vec + prep!(")");
        assert_eq!(sql.simulate(), "('A','B')");
        let sql = prep!("(") + &["A","B"][..] + prep!(")");
        assert_eq!(sql.simulate(), "('A','B')");
        let sli = &[String::from("A"),String::from("B")][..];
        let sql = prep!("(") + sli + prep!(")");
        assert_eq!(sql.simulate(), "('A','B')");
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn uuid() {
        use uuid::Uuid;
        let uuid = prep!() + Uuid::nil();
        assert_eq!(uuid.simulate(), "'00000000000000000000000000000000'");
        let uuid = prep!() + &Uuid::nil();
        assert_eq!(uuid.simulate(), "'00000000000000000000000000000000'");
        let uuid = prep!() + Uuid::parse_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap();
        assert_eq!(uuid.simulate(), "'936DA01F9ABD4D9D80C702AF85C822A8'");
        let uuid = prep!() + Uuid::new_v4();
        assert_eq!(uuid.simulate().len(), 32+2);
    }

    mod simulate {
        use crate as concatsql;
        use concatsql::prelude::*;

        #[test]
        fn double_quotaion_inside_double_quote() {
            assert_eq!(
                (prep!() + r#"".ow(""inside str"") -> String""#).simulate(),
                r#"'".ow(""inside str"") -> String"'"#
            );
            assert_eq!(
                (prep!() + r#"".ow("inside str") -> String""#).simulate(),
                r#"'".ow("inside str") -> String"'"#
            );
        }

        #[test]
        fn double_quotaion_inside_sigle_quote() {
            assert_eq!(
                (prep!() + r#""I'm Alice""#).simulate(),
                r#"'"I''m Alice"'"#
            );
            assert_eq!(
                (prep!() + r#""I''m Alice""#).simulate(),
                r#"'"I''''m Alice"'"#
            );
        }

        #[test]
        fn single_quotaion_inside_double_quote() {
            assert_eq!(
                (prep!() + r#"'.ow("inside str") -> String'"#).simulate(),
                r#"'''.ow("inside str") -> String'''"#
            );
        }

        #[test]
        fn single_quotaion_inside_sigle_quote() {
            assert_eq!(
                (prep!() + "'I''m Alice'").simulate(),
                r#"'''I''''m Alice'''"#
            );
        }

        #[test]
        fn non_quotaion_inside_sigle_quote() {
            assert_eq!(
                (prep!() + "foo'bar'foo").simulate(),
                r#"'foo''bar''foo'"#
            );
        }

        #[test]
        fn non_quotaion_inside_double_quote() {
            assert_eq!(
                (prep!() + r#"foo"bar"foo"#).simulate(),
                r#"'foo"bar"foo'"#
            );
        }

        #[test]
        fn empty_string() {
            assert_eq!(prep!().simulate(), "");
            assert_eq!(prep!("").simulate(), "");
            assert_eq!((prep!("") + "").simulate(), "''");
        }
    }
}
