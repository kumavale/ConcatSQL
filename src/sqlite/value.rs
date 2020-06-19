
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Value {
    Int(i64),
    String(String),
}

impl std::string::ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::Int(i)    => i.to_string(),
            Value::String(s) => s.to_string(),
        }
    }
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

