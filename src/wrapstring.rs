use std::ops::Add;

/// TODO
#[derive(Clone, Debug, PartialEq)]
pub enum Value<'a> {
    Null,
    I32(i32),
    I64(i64),
    I128(i128),
    F32(f32),
    F64(f64),
    Text(&'a str),
    Bytes(&'a Vec<u8>),
}

/// Wraps a [String](https://doc.rust-lang.org/std/string/struct.String.html) type.
#[derive(Clone, Debug, PartialEq)]
pub struct WrapString<'a> {
    pub(crate) query:  Vec<Option<String>>,
    pub(crate) params: Vec<Value<'a>>,
}

impl<'a> WrapString<'a> {
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

    /// Return the actual SQL statements that will be executed in the database.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::prelude::*;
    /// assert_eq!(prep!("SELECT").actual_sql(),       "\"SELECT\", []");
    /// assert_eq!(prep!("O''Reilly").actual_sql(),    "\"O''Reilly\", []");
    /// assert_eq!(prep!("\"O'Reilly\"").actual_sql(), "\"\"O'Reilly\"\", []");
    /// assert_eq!((prep!("foo")+"bar").actual_sql(),  "\"foo?\", [\"bar\"]");
    /// assert_eq!((prep!("foo")+42i32).actual_sql(),  "\"foo?\", [42]");
    /// assert_eq!((prep!("foo")+"42").actual_sql(),   "\"foo?\", [\"42\"]");
    /// assert_eq!((prep!()+"O'Reilly").actual_sql(),  "\"?\", [\"O'Reilly\"]");
    /// ```
    pub fn actual_sql(&self) -> String {
        let params = {
            let mut params = "[".to_string();
            for (i, param) in self.params.iter().enumerate() {
                if i != 0 {
                    params.push_str(", ");
                }
                match param {
                    Value::Null         => params.push_str("Null"),
                    Value::I32(value)   => params.push_str(&value.to_string()),
                    Value::I64(value)   => params.push_str(&value.to_string()),
                    Value::I128(value)  => params.push_str(&value.to_string()),
                    Value::F32(value)   => params.push_str(&value.to_string()),
                    Value::F64(value)   => params.push_str(&value.to_string()),
                    Value::Text(value)  => params.push_str(&format!("\"{}\"", value)),
                    Value::Bytes(value) => params.push_str(&format!("{:?}", value)),
                }
            }
            params.push(']');
            params
        };
        format!("\"{}\", {}", self.compile(), params)
    }

    pub(crate) fn compile(&self) -> String {
        let mut query = String::new();
        for part in &self.query {
            match part {
                Some(s) => query.push_str(&s),
                None =>    query.push('?'),
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
        self.params.push(Value::Text(Box::leak(Box::new(other)) as &'a str));
        self
    }
}

impl<'a> Add<&'a String> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'a String) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Text(other));
        self
    }
}

impl<'a> Add<&'a str> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'a str) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Text(other));
        self
    }
}

impl<'a> Add<&'a &str> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'a &str) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Text(other));
        self
    }
}

impl<'a> Add<std::borrow::Cow<'_, str>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: std::borrow::Cow<'_, str>) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Text(Box::leak(Box::new(other.into_owned()))));
        self
    }
}

impl<'a> Add<&'a std::borrow::Cow<'_, str>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'a std::borrow::Cow<'_, str>) -> WrapString<'a> {
        self.query .push(None);
        self.params.push(Value::Text(other));
        self
    }
}

impl<'a> Add<Vec<u8>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: Vec<u8>) -> WrapString<'a> {
        self.query .push(None);
        //self.params.push(Value::Text(crate::parser::to_binary_literal(&other)));
        self.params.push(Value::Bytes(Box::leak(Box::new(other)) as &'a Vec<u8>));
        self
    }
}

impl<'a> Add<&'a Vec<u8>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'a Vec<u8>) -> WrapString<'a> {
        self.query .push(None);
        //self.params.push(Value::Text(crate::parser::to_binary_literal(other)));
        self.params.push(Value::Bytes(other));
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

macro_rules! impl_add_I128_for_WrapString {
    ( $($t:ty),* ) => ($(
        impl<'a> Add<$t> for WrapString<'a> {
            type Output = WrapString<'a>;
            #[inline]
            fn add(mut self, other: $t) -> WrapString<'a> {
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
    &'a str,
    std::borrow::Cow<'_, str>,
    Vec<u8>,
    u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize,
    f32, f64,
}

///// A trait for converting a value to a [WrapString<'a>](./struct.WrapString<'a>.html).
/// TODO
pub trait IntoWrapString<'a> {
    ///// Converts the given value to a [WrapString<'a>](./struct.WrapString<'a>.html).
    /// TODO
    fn into_wrapstring(self) -> WrapString<'a>;
}

impl<'a> IntoWrapString<'a> for WrapString<'a> {
    fn into_wrapstring(self) -> WrapString<'a> {
        self
    }
}

impl<'a> IntoWrapString<'a> for &WrapString<'a> {
    fn into_wrapstring(self) -> WrapString<'a> {
        self.clone()
    }
}

impl<'a> IntoWrapString<'a> for &'static str {
    fn into_wrapstring(self) -> WrapString<'a> {
        WrapString::new(self)
    }
}

impl<'a> Drop for WrapString<'a> {
    fn drop(&mut self) {
        for param in &self.params {
            match param {
                Value::Text(value)  => { let _ = unsafe { Box::from_raw(value as *const _ as *mut &str) }; }
                Value::Bytes(value) => { let _ = unsafe { Box::from_raw(value as *const _ as *mut Vec<u8>) }; }
                _ => ()
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use crate as concatsql;
    use concatsql::prelude::*;

    #[test]
    #[allow(non_snake_case)]
    #[allow(clippy::op_ref, clippy::deref_addrof, clippy::identity_op, clippy::approx_constant)]
    fn concat_anything_type() {
        use std::borrow::Cow;
        let E = String::from("E");
        let sql: WrapString = prep!("A") + prep!("B") + "C" + String::from("D") + &E + &prep!("F") + 42 + 3.14;
        assert_eq!(sql.actual_sql(), "\"AB???F??\", [\"C\", \"D\", \"E\", 42, 3.14]");
        let sql = prep!() + "A" + &"B" + *&&"C" + **&&&"D";
        assert_eq!(sql.actual_sql(), "\"????\", [\"A\", \"B\", \"C\", \"D\"]");
        let sql = prep!() + 0usize + 1u8 + 2u16 + 3u32 + 4u64 + 5u128 + 6isize + 7i8 + 8i16 + 9i32 + 0i64 + 1i128 + 2f32 + 3f64;
        assert_eq!(sql.actual_sql(), "\"??????????????\", [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3]");
        let sql = prep!() + f32::MAX + f32::INFINITY + f32::NAN;
        assert_eq!(sql.actual_sql(), "\"???\", [340282350000000000000000000000000000000, inf, NaN]");
        let bytes = vec![0,1,2];
        let sql = prep!() + vec![b'A',b'B',b'C'] + &bytes;
        if cfg!(feature = "sqlite") || cfg!(feature = "mysql") {
            assert_eq!(sql.actual_sql(), "\"??\", [[65, 66, 67], [0, 1, 2]]");
        } else {
            assert_eq!(sql.actual_sql(), "\"$1$2\", [[65, 66, 67], [0, 1, 2]]");
        }
        let CowOwnedD = Cow::Owned("D".to_string());
        let sql = prep!() + Cow::Borrowed("A") + &Cow::Borrowed("B") + Cow::Owned("C".to_string()) + &CowOwnedD;
        assert_eq!(sql.actual_sql(), "\"????\", [\"A\", \"B\", \"C\", \"D\"]");
        let x: Option<i32> = Some(42);
        let y: Option<i32> = None;
        let sql = prep!("A") + Some("B") + Some(String::from("C")) + Some(0i32) + Some(3.14f32) + x + y;
        assert_eq!(sql.actual_sql(), "\"A??????\", [\"B\", \"C\", 0, 3.14, 42, Null]");
    }

    mod actual_sql {
        use crate as concatsql;
        use concatsql::prelude::*;

        #[test]
        fn double_quotaion_inside_double_quote() {
            assert_eq!(
                (prep!() + r#"".ow(""inside str"") -> String""#).actual_sql(),
                r#""?", ["".ow(""inside str"") -> String""]"#
            );
            assert_eq!(
                (prep!() + r#"".ow("inside str") -> String""#).actual_sql(),
                r#""?", ["".ow("inside str") -> String""]"#
            );
        }

        #[test]
        fn double_quotaion_inside_sigle_quote() {
            assert_eq!(
                (prep!() + r#""I'm Alice""#).actual_sql(),
                r#""?", [""I'm Alice""]"#
            );
            assert_eq!(
                (prep!() + r#""I''m Alice""#).actual_sql(),
                r#""?", [""I''m Alice""]"#
            );
        }

        #[test]
        fn single_quotaion_inside_double_quote() {
            assert_eq!(
                (prep!() + r#"'.ow("inside str") -> String'"#).actual_sql(),
                r#""?", ["'.ow("inside str") -> String'"]"#
            );
        }

        #[test]
        fn single_quotaion_inside_sigle_quote() {
            assert_eq!(
                (prep!() + "'I''m Alice'").actual_sql(),
                r#""?", ["'I''m Alice'"]"#
            );
        }

        #[test]
        fn non_quotaion_inside_sigle_quote() {
            assert_eq!(
                (prep!() + "foo'bar'foo").actual_sql(),
                r#""?", ["foo'bar'foo"]"#
            );
        }

        #[test]
        fn non_quotaion_inside_double_quote() {
            assert_eq!(
                (prep!() + r#"foo"bar"foo"#).actual_sql(),
                r#""?", ["foo"bar"foo"]"#
            );
        }

        #[test]
        fn empty_string() {
            assert_eq!(prep!().actual_sql(), "\"\", []");
            assert_eq!(prep!("").actual_sql(), "\"\", []");
            assert_eq!((prep!("") + "").actual_sql(), "\"?\", [\"\"]");
        }
    }
}
