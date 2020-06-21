
/// Enum listing possible errors from owsql.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum OwsqlError {
    /// The error code.
    Code(isize),
    /// The error message.
    Message(String),
    /// An any errors.
    AnyError,
}

impl OwsqlError {
    pub(crate) fn new<T: Clone + ToString>(msg: T) -> Self {
        OwsqlError::Message(msg.to_string())
    }
}

impl std::string::ToString for OwsqlError {
    fn to_string(&self) -> String {
        match self {
            OwsqlError::Code(i) =>    i.to_string(),
            OwsqlError::Message(s) => s.to_string(),
            OwsqlError::AnyError =>   String::from("AnyError"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owsql_error() {
        assert_eq!(OwsqlError::Code(0).to_string(), "0");
        assert_eq!(OwsqlError::Message("test".to_string()).to_string(), "test");
        assert_eq!(OwsqlError::new("test").to_string(), "test");
        assert_eq!(OwsqlError::new("test".to_string()).to_string(), "test");
        assert_eq!(OwsqlError::new(42).to_string(), "42");
    }
}

