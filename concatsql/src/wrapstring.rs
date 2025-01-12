use std::borrow::Cow;
use std::net::IpAddr;
use std::ops::Add;
use std::time::SystemTime;
use uuid::Uuid;

use crate::connection::ConnKind;
use crate::parser::{escape_string, to_binary_literal};
use crate::value::{SystemTimeToString, ToValue, Value};

/// Wraps a [String](https://doc.rust-lang.org/std/string/struct.String.html) type.
#[derive(Clone, Debug, PartialEq)]
pub struct WrapString<'a> {
    pub(crate) query: Vec<Option<Cow<'a, str>>>,
    pub(crate) params: Vec<Value<'a>>,
}

impl<'a> WrapString<'a> {
    #[doc(hidden)]
    #[inline]
    pub fn _init(query: Vec<Option<&'static str>>, params: Vec<Value<'a>>) -> Self {
        Self {
            query: query.iter().map(|q| q.map(Cow::from)).collect(),
            params,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn init(s: &'static str) -> Self {
        Self {
            query: vec![Some(Cow::Borrowed(s))],
            params: Vec::new(),
        }
    }

    #[doc(hidden)]
    #[inline]
    pub const fn null() -> Self {
        Self {
            query: Vec::new(),
            params: Vec::new(),
        }
    }

    #[inline]
    pub(crate) fn new<T: ?Sized + ToString>(s: &T) -> Self {
        Self {
            query: vec![Some(Cow::Owned(s.to_string()))],
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
    /// # use concatsql::prep;
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
                Some(s) => query.push_str(s),
                None => {
                    match &self.params[index] {
                        Value::Null => query.push_str("NULL"),
                        Value::I32(value) => query.push_str(&value.to_string()),
                        Value::I64(value) => query.push_str(&value.to_string()),
                        Value::F32(value) => query.push_str(&value.to_string()),
                        Value::F64(value) => query.push_str(&value.to_string()),
                        Value::Text(value) => query.push_str(&escape_string(value)),
                        Value::Bytes(value) => query.push_str(&to_binary_literal(value)),
                        Value::IpAddr(value) => query.push_str(&format!("'{}'", value)),
                        Value::Time(value) => query.push_str(&format!("'{}'", value.to_string())),
                    }
                    index += 1;
                }
            }
        }
        query
    }

    /// Returns the length of a string other than a placeholders.
    pub fn len(&self) -> usize {
        self.query.iter().flatten().map(|part| part.len()).sum()
    }

    /// Returns the query's vector length.
    pub fn query_len(&self) -> usize {
        self.query.len()
    }

    /// Returns the params's vector length.
    pub fn params_len(&self) -> usize {
        self.params.len()
    }

    /// Truncates this WrapString, removing all contents.
    pub fn clear(&mut self) {
        self.query.clear();
        self.params.clear();
    }

    /// Returns true if this WrapString has a length of zero, and false otherwise.
    pub fn is_empty(&self) -> bool {
        self.query.is_empty() && self.params.is_empty()
    }

    /// Organize the query field of WrapString.
    ///
    /// # Likes
    ///
    /// ```ignore
    /// // Before squash
    /// WrapString {
    ///     query: [Some("a"),Some("b"),Some("c"),None,Some("1"),Some("2")],
    ///     params: [],
    /// }
    ///
    /// // After squash
    /// WrapString {
    ///     query: [Some("abc"),None,Some("12")],
    ///     params: [],
    /// }
    /// ```
    pub fn squash(&mut self) {
        let mut new_query = Vec::new();
        let mut new_part = String::new();
        for part in &self.query {
            if let Some(part) = part {
                new_part.push_str(part);
            } else {
                new_query.push(Some(Cow::Owned(std::mem::take(&mut new_part))));
                new_query.push(None);
            }
        }
        if !new_part.is_empty() {
            new_query.push(Some(Cow::Owned(new_part)));
        }
        self.query = new_query;
    }
}

impl<'a> Add for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: WrapString<'a>) -> WrapString<'a> {
        self.query.extend_from_slice(&other.query);
        self.params.extend_from_slice(&other.params);
        self
    }
}

impl<'a, 'b> Add<&'b WrapString<'a>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'b WrapString<'a>) -> WrapString<'a> {
        self.query.extend_from_slice(&other.query);
        self.params.extend_from_slice(&other.params);
        self
    }
}

impl<'a> Add<String> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: String) -> WrapString<'a> {
        self.query.push(None);
        self.params.push(Value::Text(Cow::Owned(other)));
        self
    }
}

impl<'a> Add<&'a String> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'a String) -> WrapString<'a> {
        self.query.push(None);
        self.params.push(Value::Text(Cow::Borrowed(other)));
        self
    }
}

impl<'a> Add<&'a str> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'a str) -> WrapString<'a> {
        self.query.push(None);
        self.params.push(Value::Text(Cow::Borrowed(other)));
        self
    }
}

impl<'a> Add<&'a &str> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'a &str) -> WrapString<'a> {
        self.query.push(None);
        self.params.push(Value::Text(Cow::Borrowed(other)));
        self
    }
}

impl<'a> Add<std::borrow::Cow<'a, str>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: std::borrow::Cow<'a, str>) -> WrapString<'a> {
        self.query.push(None);
        self.params.push(Value::Text(other));
        self
    }
}

impl<'a> Add<&'a std::borrow::Cow<'a, str>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &'a std::borrow::Cow<'a, str>) -> WrapString<'a> {
        self.query.push(None);
        self.params.push(Value::Text(Cow::Borrowed(other)));
        self
    }
}

impl<'a> Add<Vec<u8>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: Vec<u8>) -> WrapString<'a> {
        self.query.push(None);
        self.params.push(Value::Bytes(other));
        self
    }
}

impl<'a> Add<&Vec<u8>> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &Vec<u8>) -> WrapString<'a> {
        self.query.push(None);
        self.params.push(Value::Bytes(other.clone()));
        self
    }
}

impl<'a> Add<&[u8]> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &[u8]) -> WrapString<'a> {
        self.query.push(None);
        self.params.push(Value::Bytes(other.to_vec()));
        self
    }
}

impl<'a> Add<&[&(dyn ToValue<'a>)]> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &[&(dyn ToValue<'a>)]) -> WrapString<'a> {
        if let Some(first) = other.first() {
            self.query.push(None);
            self.params.push(first.to_value());
        }
        for param in other.iter().skip(1) {
            self.query.push(Some(Cow::Borrowed(",")));
            self.query.push(None);
            self.params.push(param.to_value());
        }
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
        self.query.push(None);
        self.params
            .push(Value::Text(Cow::Owned(format!("{:X}", other.simple()))));
        self
    }
}

/// Sent as a 32-byte string.
impl<'a> Add<&Uuid> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &Uuid) -> WrapString<'a> {
        self.query.push(None);
        self.params
            .push(Value::Text(Cow::Owned(format!("{:X}", other.simple()))));
        self
    }
}

impl<'a> Add<IpAddr> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: IpAddr) -> WrapString<'a> {
        self.query.push(None);
        self.params.push(Value::IpAddr(other));
        self
    }
}

impl<'a> Add<&IpAddr> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &IpAddr) -> WrapString<'a> {
        self.query.push(None);
        self.params.push(Value::IpAddr(*other));
        self
    }
}

impl<'a> Add<SystemTime> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: SystemTime) -> WrapString<'a> {
        self.query.push(None);
        self.params.push(Value::Time(other));
        self
    }
}

impl<'a> Add<&SystemTime> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: &SystemTime) -> WrapString<'a> {
        self.query.push(None);
        self.params.push(Value::Time(*other));
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
        self.query.push(None);
        self.params.push(Value::F32(other));
        self
    }
}

impl<'a> Add<f64> for WrapString<'a> {
    type Output = WrapString<'a>;
    #[inline]
    fn add(mut self, other: f64) -> WrapString<'a> {
        self.query.push(None);
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
        self.query.push(None);
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
/// # use concatsql::prep;
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
            self.query.push(None);
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
        /// # use concatsql::prep;
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

impl_add_arrays_borrowed_for_WrapString! {
    Vec<&'a str>,
    &'a Vec<String>,
    &'a Vec<&'a str>,
    &'a [&'a str],
    &'a [String],
}

/// A trait for converting that can be converted to [`WrapString`].
pub trait IntoWrapString<'a> {
    #[doc(hidden)]
    fn compile(&self, kind: ConnKind) -> Cow<'a, str>;
    #[doc(hidden)]
    fn params(&self) -> &[Value<'a>];
}

macro_rules! compile {
    ($self: expr, $kind: expr) => {
        match $kind {
            #[cfg(feature = "sqlite")]
            ConnKind::SQLite => {
                let mut query = String::with_capacity(
                    $self
                        .query
                        .iter()
                        .map(|q| q.as_ref().map_or(1, |q| q.len()))
                        .sum(),
                );
                for part in &$self.query {
                    match part {
                        Some(s) => query.push_str(s),
                        None => query.push('?'),
                    }
                }
                Cow::Owned(query)
            }
            #[cfg(feature = "mysql")]
            ConnKind::MySQL => {
                let mut query = String::with_capacity(
                    $self
                        .query
                        .iter()
                        .map(|q| q.as_ref().map_or(1, |q| q.len()))
                        .sum(),
                );
                for part in &$self.query {
                    match part {
                        Some(s) => query.push_str(s),
                        None => query.push('?'),
                    }
                }
                Cow::Owned(query)
            }
            #[cfg(feature = "postgres")]
            ConnKind::PostgreSQL => {
                let mut query = String::with_capacity(
                    $self
                        .query
                        .iter()
                        .map(|q| q.as_ref().map_or(3, |q| q.len()))
                        .sum(),
                );
                let mut index = 1;
                for part in &$self.query {
                    match part {
                        Some(s) => query.push_str(s),
                        None => {
                            query.push_str(&format!("${}", index));
                            index += 1;
                        }
                    }
                }
                Cow::Owned(query)
            }
        }
    };
}

impl<'a> IntoWrapString<'a> for WrapString<'a> {
    #[doc(hidden)]
    #[inline]
    fn compile(&self, kind: ConnKind) -> Cow<'a, str> {
        compile!(self, kind)
    }

    #[doc(hidden)]
    #[inline]
    fn params(&self) -> &[Value<'a>] {
        &self.params
    }
}

impl<'a, 'b> IntoWrapString<'a> for &'b WrapString<'a> {
    #[doc(hidden)]
    #[inline]
    fn compile(&self, kind: ConnKind) -> Cow<'a, str> {
        compile!(self, kind)
    }

    #[doc(hidden)]
    #[inline]
    fn params(&self) -> &[Value<'a>] {
        &self.params
    }
}

impl<'a> IntoWrapString<'a> for &'static str {
    #[doc(hidden)]
    #[inline]
    fn compile(&self, _kind: ConnKind) -> Cow<'a, str> {
        Cow::Borrowed(self)
    }

    #[doc(hidden)]
    #[inline]
    fn params(&self) -> &[Value<'a>] {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use crate as concatsql;
    use concatsql::prelude::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::time::UNIX_EPOCH;

    #[test]
    #[allow(
        clippy::op_ref,
        clippy::deref_addrof,
        clippy::identity_op,
        clippy::approx_constant,
        clippy::many_single_char_names
    )]
    fn concat_anything_type() {
        use std::borrow::Cow;
        let a = String::from("A");
        let b = &String::from("B");
        let c = &**&&String::from("C");
        let d = &***&&&String::from("D");
        let e = String::from("E");
        let sql: WrapString =
            query!("A") + query!("B") + "C" + String::from("D") + &e + &query!("F") + 42 + 3.14;
        assert_eq!(sql.simulate(), "AB'C''D''E'F423.14");
        let sql = query!("") + a + b + c + d;
        assert_eq!(sql.simulate(), "'A''B''C''D'");
        let sql = query!("") + "A" + &"B" + *&&"C" + **&&&"D";
        assert_eq!(sql.simulate(), "'A''B''C''D'");
        let sql = query!("")
            + 0usize
            + 1u8
            + 2u16
            + 3u32
            + 4u64
            + 5isize
            + 6i8
            + 7i16
            + 8i32
            + 9i64
            + 0f32
            + 1f64;
        assert_eq!(sql.simulate(), "012345678901");
        let sql = query!("") + f32::MAX + f32::INFINITY + f32::NAN;
        assert_eq!(
            sql.simulate(),
            "340282350000000000000000000000000000000infNaN"
        );
        let sql = query!("") + vec![b'A', b'B', b'C'] + &vec![0, 1, 2];
        if cfg!(feature = "sqlite") || cfg!(feature = "mysql") {
            assert_eq!(sql.simulate(), "X'414243'X'000102'");
        } else {
            assert_eq!(sql.simulate(), "'\\x414243''\\x000102'");
        }
        let sql =
            query!("") + Cow::Borrowed("A") + &Cow::Borrowed("B") + Cow::Owned("C".to_string());
        assert_eq!(sql.simulate(), "'A''B''C'");
        let sql = query!("A")
            + Some("B")
            + Some(String::from("C"))
            + Some(0i32)
            + Some(3.14f32)
            + Some(42i32)
            + None as Option<i32>
            + ();
        assert_eq!(sql.simulate(), "A'B''C'03.1442NULLNULL");
        let vec: Vec<String> = Vec::new();
        let sql = query!("(") + vec + query!(")");
        assert_eq!(sql.simulate(), "(NULL)");
        let sql = query!("(") + vec!["A"] + query!(")");
        assert_eq!(sql.simulate(), "('A')");
        let sql = query!("(") + vec!["A", "B"] + query!(")");
        assert_eq!(sql.simulate(), "('A','B')");
        let sql = query!("(") + vec![String::from("A"), String::from("B")] + query!(")");
        assert_eq!(sql.simulate(), "('A','B')");
        let vec = vec!["A", "B"];
        let sql = query!("(") + &vec + query!(")");
        assert_eq!(sql.simulate(), "('A','B')");
        let vec = vec![String::from("A"), String::from("B")];
        let sql = query!("(") + &vec + query!(")");
        assert_eq!(sql.simulate(), "('A','B')");
        let sql = query!("(") + &["A", "B"][..] + query!(")");
        assert_eq!(sql.simulate(), "('A','B')");
        let sli = &[String::from("A"), String::from("B")][..];
        let sql = query!("(") + sli + query!(")");
        assert_eq!(sql.simulate(), "('A','B')");
        let sql = query!("") + IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(sql.simulate(), "'127.0.0.1'");
        let sql = query!("") + IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
        assert_eq!(sql.simulate(), "'::1'");
        let sql = query!("") + UNIX_EPOCH;
        assert_eq!(sql.simulate(), "'1970-01-01 00:00:00.000000000'");
    }

    #[test]
    fn params() {
        let sql = query!("")
            + params![
                (),
                42i8,
                42i16,
                42i32,
                0.1f32,
                2.3f64,
                String::from("A"),
                "B",
                IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
                UNIX_EPOCH,
            ];
        assert_eq!(
            sql.simulate(),
            "NULL,42,42,42,0.1,2.3,'A','B','::1','1970-01-01 00:00:00.000000000'"
        );
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn uuid() {
        use uuid::Uuid;
        let uuid = query!("") + Uuid::nil();
        assert_eq!(uuid.simulate(), "'00000000000000000000000000000000'");
        let uuid = query!("") + &Uuid::nil();
        assert_eq!(uuid.simulate(), "'00000000000000000000000000000000'");
        let uuid = query!("") + Uuid::parse_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap();
        assert_eq!(uuid.simulate(), "'936DA01F9ABD4D9D80C702AF85C822A8'");
        let uuid = query!("") + Uuid::new_v4();
        assert_eq!(uuid.simulate().len(), 32 + 2);
    }

    #[test]
    fn len() {
        assert_eq!((query!("ABC") + query!("123")).len(), 6);
        let sql: WrapString = query!("ABC") + 42 + query!("123");
        assert_eq!(sql.len(), 6);
        assert_eq!(query!("").len(), 0);
    }

    #[test]
    fn query_len() {
        assert_eq!((query!("ABC") + query!("123")).query_len(), 2);
        let sql: WrapString = query!("ABC") + 42 + query!("123");
        assert_eq!(sql.query_len(), 3);
        assert_eq!(query!("").query_len(), 0);
    }

    #[test]
    fn params_len() {
        assert_eq!((query!("ABC") + query!("123")).params_len(), 0);
        let sql: WrapString = query!("ABC") + 42 + query!("123");
        assert_eq!(sql.params_len(), 1);
        assert_eq!(query!("").params_len(), 0);
    }

    #[test]
    fn clear() {
        let mut sql: WrapString = query!("ABC") + 42 + query!("123");
        assert_eq!(sql.query_len(), 3);
        assert_eq!(sql.params_len(), 1);
        sql.clear();
        assert_eq!(sql.query_len(), 0);
        assert_eq!(sql.params_len(), 0);
    }

    #[test]
    fn is_empty() {
        assert!(query!("").is_empty());
    }

    #[test]
    fn squash() {
        let mut sql: WrapString =
            query!("A") + query!("B") + 42 + query!("1") + query!("2") + query!("3");
        assert_eq!(sql.query_len(), 6);
        assert_eq!(sql.params_len(), 1);
        sql.squash();
        assert_eq!(sql.query_len(), 3);
        assert_eq!(sql.params_len(), 1);
    }

    mod simulate {
        use crate as concatsql;
        use concatsql::prelude::*;

        #[test]
        fn double_quotaion_inside_double_quote() {
            assert_eq!(
                (query!("") + r#"".ow(""inside str"") -> String""#).simulate(),
                r#"'".ow(""inside str"") -> String"'"#
            );
            assert_eq!(
                (query!("") + r#"".ow("inside str") -> String""#).simulate(),
                r#"'".ow("inside str") -> String"'"#
            );
        }

        #[test]
        fn double_quotaion_inside_sigle_quote() {
            assert_eq!(
                (query!("") + r#""I'm Alice""#).simulate(),
                r#"'"I''m Alice"'"#
            );
            assert_eq!(
                (query!("") + r#""I''m Alice""#).simulate(),
                r#"'"I''''m Alice"'"#
            );
        }

        #[test]
        fn single_quotaion_inside_double_quote() {
            assert_eq!(
                (query!("") + r#"'.ow("inside str") -> String'"#).simulate(),
                r#"'''.ow("inside str") -> String'''"#
            );
        }

        #[test]
        fn single_quotaion_inside_sigle_quote() {
            assert_eq!(
                (query!("") + "'I''m Alice'").simulate(),
                r#"'''I''''m Alice'''"#
            );
        }

        #[test]
        fn non_quotaion_inside_sigle_quote() {
            assert_eq!(
                (query!("") + "foo'bar'foo").simulate(),
                r#"'foo''bar''foo'"#
            );
        }

        #[test]
        fn non_quotaion_inside_double_quote() {
            assert_eq!(
                (query!("") + r#"foo"bar"foo"#).simulate(),
                r#"'foo"bar"foo'"#
            );
        }

        #[test]
        fn empty_string() {
            assert_eq!(query!("").simulate(), "");
            assert_eq!(query!("").simulate(), "");
            assert_eq!((query!("") + "").simulate(), "''");
        }
    }
}
