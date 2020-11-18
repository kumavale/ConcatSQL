use std::ops::Add;
use crate::parser::{escape_string, to_binary_literal};

/// Values that can be bound as static placeholders.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    I32(i32),
    I64(i64),
    I128(i128),
    F32(f32),
    F64(f64),
    Text(String),
    Bytes(Vec<u8>),
}

/// Wraps a [String](https://doc.rust-lang.org/std/string/struct.String.html) type.
#[derive(Clone, Debug, PartialEq)]
pub struct WrapString {
    pub(crate) query:  Vec<Option<String>>,
    pub(crate) params: Vec<Value>,
}

impl WrapString {
    #[doc(hidden)]
    pub fn init(s: &'static str) -> Self {
        Self {
            query:  vec![ Some(s.to_string()) ],
            params: Vec::new(),
        }
    }

    pub(crate) fn new<T: ?Sized + ToString>(s: &T) -> Self {
        Self {
            query:  vec![ Some(s.to_string()) ],
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
                        Value::I128(value)  => query.push_str(&value.to_string()),
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

impl Add for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(mut self, other: WrapString) -> WrapString {
        self.query .extend_from_slice(&other.query);
        self.params.extend_from_slice(&other.params);
        self
    }
}

impl<'a> Add<&'a WrapString> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(mut self, other: &'a WrapString) -> WrapString {
        self.query .extend_from_slice(&other.query);
        self.params.extend_from_slice(&other.params);
        self
    }
}

impl Add<String> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(mut self, other: String) -> WrapString {
        self.query .push(None);
        self.params.push(Value::Text(other));
        self
    }
}

impl Add<&String> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(mut self, other: &String) -> WrapString {
        self.query .push(None);
        self.params.push(Value::Text(other.to_string()));
        self
    }
}

impl Add<&str> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(mut self, other: &str) -> WrapString {
        self.query .push(None);
        self.params.push(Value::Text(other.to_string()));
        self
    }
}

impl Add<&&str> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(mut self, other: &&str) -> WrapString {
        self.query .push(None);
        self.params.push(Value::Text(other.to_string()));
        self
    }
}

impl Add<std::borrow::Cow<'_, str>> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(mut self, other: std::borrow::Cow<'_, str>) -> WrapString {
        self.query .push(None);
        self.params.push(Value::Text(other.into_owned()));
        self
    }
}

impl Add<&std::borrow::Cow<'_, str>> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(mut self, other: &std::borrow::Cow<'_, str>) -> WrapString {
        self.query .push(None);
        self.params.push(Value::Text(other.to_string()));
        self
    }
}

impl Add<Vec<u8>> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(mut self, other: Vec<u8>) -> WrapString {
        self.query .push(None);
        self.params.push(Value::Bytes(other));
        self
    }
}

impl Add<&Vec<u8>> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(mut self, other: &Vec<u8>) -> WrapString {
        self.query .push(None);
        self.params.push(Value::Bytes(other.clone()));
        self
    }
}

macro_rules! impl_add_I32_for_WrapString {
    ( $($t:ty),* ) => ($(
        impl Add<$t> for WrapString {
            type Output = WrapString;
            #[inline]
            fn add(mut self, other: $t) -> WrapString {
                self.query .push(None);
                self.params.push(Value::I32(other as i32));
                self
            }
        }
    )*)
}

macro_rules! impl_add_I64_for_WrapString {
    ( $($t:ty),* ) => ($(
        impl Add<$t> for WrapString {
            type Output = WrapString;
            #[inline]
            fn add(mut self, other: $t) -> WrapString {
                self.query .push(None);
                self.params.push(Value::I64(other as i64));
                self
            }
        }
    )*)
}

macro_rules! impl_add_I128_for_WrapString {
    ( $($t:ty),* ) => ($(
        impl Add<$t> for WrapString {
            type Output = WrapString;
            #[inline]
            fn add(mut self, other: $t) -> WrapString {
                self.query .push(None);
                self.params.push(Value::I128(other as i128));
                self
            }
        }
    )*)
}

impl_add_I32_for_WrapString!(u8, u16, u32, i8, i16, i32);
impl_add_I64_for_WrapString!(u64, i64);
impl_add_I128_for_WrapString!(u128, i128);

#[cfg(target_pointer_width = "16")]
#[cfg(target_pointer_width = "32")]
impl_add_I32_for_WrapString!(usize, isize);

#[cfg(target_pointer_width = "64")]
impl_add_I64_for_WrapString!(usize, isize);

impl Add<f32> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(mut self, other: f32) -> WrapString {
        self.query .push(None);
        self.params.push(Value::F32(other));
        self
    }
}

impl Add<f64> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(mut self, other: f64) -> WrapString {
        self.query .push(None);
        self.params.push(Value::F64(other));
        self
    }
}

macro_rules! impl_add_Option_for_WrapString {
    ( $($t:ty),* ) => {$(
        impl Add<Option<$t>> for WrapString {
            type Output = WrapString;
            #[inline]
            fn add(mut self, other: Option<$t>) -> WrapString {
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
    &str,
    std::borrow::Cow<'_, str>,
    Vec<u8>,
    u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize,
    f32, f64,
}

impl Add<()> for WrapString {
    type Output = WrapString;
    #[inline]
    fn add(mut self, _other: ()) -> WrapString {
        self.query .push(None);
        self.params.push(Value::Null);
        self
    }
}

/// A trait for converting a value to a [WrapString](./struct.WrapString.html).
pub trait IntoWrapString {
    /// Converts the given value to a [WrapString](./struct.WrapString.html).
    fn into_wrapstring(self) -> WrapString;
}

impl IntoWrapString for WrapString {
    fn into_wrapstring(self) -> WrapString {
        self
    }
}

impl IntoWrapString for &WrapString {
    fn into_wrapstring(self) -> WrapString {
        self.clone()
    }
}

impl IntoWrapString for &'static str {
    fn into_wrapstring(self) -> WrapString {
        WrapString::new(self)
    }
}


#[cfg(test)]
mod tests {
    use crate as concatsql;
    use concatsql::prelude::*;

    #[test]
    #[allow(clippy::op_ref, clippy::deref_addrof, clippy::identity_op, clippy::approx_constant)]
    fn concat_anything_type() {
        use std::borrow::Cow;
        let sql: WrapString = prep!("A") + prep!("B") + "C" + String::from("D") + &String::from("E") + &prep!("F") + 42 + 3.14;
        assert_eq!(sql.simulate(), "AB'C''D''E'F423.14");
        let sql = prep!() + String::from("A") + &String::from("B") + *&&String::from("C") + **&&&String::from("D");
        assert_eq!(sql.simulate(), "'A''B''C''D'");
        let sql = prep!() + "A" + &"B" + *&&"C" + **&&&"D";
        assert_eq!(sql.simulate(), "'A''B''C''D'");
        let sql = prep!() + 0usize + 1u8 + 2u16 + 3u32 + 4u64 + 5u128 + 6isize + 7i8 + 8i16 + 9i32 + 0i64 + 1i128 + 2f32 + 3f64;
        assert_eq!(sql.simulate(), "01234567890123");
        let sql = prep!() + f32::MAX + f32::INFINITY + f32::NAN;
        assert_eq!(sql.simulate(), "340282350000000000000000000000000000000infNaN");
        let sql = prep!() + vec![b'A',b'B',b'C'] + &vec![0,1,2];
        if cfg!(feature = "sqlite") || cfg!(feature = "mysql") {
            assert_eq!(sql.simulate(), "X'414243'X'000102'");
        } else {
            assert_eq!(sql.simulate(), "'\\x414243''\\x000102'");
        }
        let sql = prep!() + Cow::Borrowed("A") + &Cow::Borrowed("B") + Cow::Owned("C".to_string()) + &Cow::Owned("D".to_string());
        assert_eq!(sql.simulate(), "'A''B''C''D'");
        let sql = prep!("A") + Some("B") + Some(String::from("C")) + Some(0i32) + Some(3.14f32) + Some(42i32) + None as Option<i32> + ();
        assert_eq!(sql.simulate(), "A'B''C'03.1442NULLNULL");
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
