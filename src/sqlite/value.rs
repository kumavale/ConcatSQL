
#[derive(Clone, PartialEq, PartialOrd)]
#[derive(Debug)]
pub enum Value {
    Int(i64),
    String(String),
}

impl From<i64> for Value {
    fn from(x: i64) -> Value {
        Value::Int(x)
    }
}

impl From<&str> for Value {
    fn from(x: &str) -> Value {
        Value::String(x.to_string())
    }
}

