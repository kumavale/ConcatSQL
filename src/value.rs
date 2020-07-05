
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Value {
    Int(i64),
    Float(f64),
    Char(char),
    String(String),
}

impl std::string::ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::Int(i)    => i.to_string(),
            Value::Float(f)   => f.to_string(),
            Value::Char(c)   => c.to_string(),
            Value::String(s) => s.to_string(),
        }
    }
}

impl From<i64> for Value {
    fn from(x: i64) -> Value {
        Value::Int(x)
    }
}

impl From<f64> for Value {
    fn from(x: f64) -> Value {
        Value::Float(x)
    }
}

impl From<char> for Value {
    fn from(x: char) -> Value {
        Value::Char(x)
    }
}

impl From<&str> for Value {
    fn from(x: &str) -> Value {
        Value::String(x.to_string())
    }
}

