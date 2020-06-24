
/// Enum listing possible errors from owsql.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum OwsqlError {
    /// The error message.
    Message(String),
    /// An any errors.
    AnyError,
}

/// Change the output error message.
#[derive(Debug, PartialEq)]
pub enum OwsqlErrorLevel {
    /// No error message returned, always return Result::Ok(T).
    AlwaysOk,
    /// This is the level that should be set at release.
    Release,
    /// This is the level that should be set during development.
    Develop,
    /// Output more detailed messages during development.
    Debug,
}

impl Default for OwsqlErrorLevel {
    fn default() -> Self {
        if cfg!(debug_assertions) {
            OwsqlErrorLevel::Develop
        } else {
            OwsqlErrorLevel::Release
        }
    }
}

impl OwsqlError {
    pub(crate) fn new<T: Clone + ToString>(msg: T) -> Self {
        OwsqlError::Message(msg.to_string())
    }
}

impl std::string::ToString for OwsqlError {
    fn to_string(&self) -> String {
        match self {
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
        assert_eq!(OwsqlError::Message("test".to_string()).to_string(), "test");
        assert_eq!(OwsqlError::new("test").to_string(), "test");
        assert_eq!(OwsqlError::new("test".to_string()).to_string(), "test");
        assert_eq!(OwsqlError::new(42).to_string(), "42");
    }
}

