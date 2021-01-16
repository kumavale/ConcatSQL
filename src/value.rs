use std::borrow::Cow;

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

/// A trait for types that can be converted into Database values.
pub trait ToValue<'a> {
    fn to_value(&self) -> Value<'a>;
}

impl<'a> ToValue<'a> for () {
    fn to_value(&self) -> Value<'a> {
        Value::Null
    }
}

macro_rules! impl_to_value_for_i32 {
    ( $($t:ty),* ) => {$(
        impl<'a> ToValue<'a> for $t {
            fn to_value(&self) -> Value<'a> {
                Value::I32(*self as i32)
            }
        }
    )*};
    ( $($t:ty,)* ) => { impl_to_value_for_i32!{ $( $t ),* } }
}

impl_to_value_for_i32! {
    i8, i16, i32,
}

impl<'a> ToValue<'a> for i64 {
    fn to_value(&self) -> Value<'a> {
        Value::I64(*self)
    }
}

impl<'a> ToValue<'a> for f32 {
    fn to_value(&self) -> Value<'a> {
        Value::F32(*self)
    }
}

impl<'a> ToValue<'a> for f64 {
    fn to_value(&self) -> Value<'a> {
        Value::F64(*self)
    }
}

impl<'a> ToValue<'a> for String {
    fn to_value(&self) -> Value<'a> {
        Value::Text(Cow::Owned(self.to_string()))
    }
}

impl<'a> ToValue<'a> for &'a str {
    fn to_value(&self) -> Value<'a> {
        Value::Text(Cow::Borrowed(self))
    }
}

impl<'a> ToValue<'a> for Vec<u8> {
    fn to_value(&self) -> Value<'a> {
        Value::Bytes(self.clone())
    }
}

impl<'a> ToValue<'a> for &'a Vec<u8> {
    fn to_value(&self) -> Value<'a> {
        Value::Bytes(self.to_vec())
    }
}

