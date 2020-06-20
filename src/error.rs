
/// Enum listing possible errors from owsql.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum OwsqlError {
    /// The error code.
    Code(isize),
    /// The error message.
    Message(String),
    /// The empty tuple like error.
    Err(()),
}

impl std::string::ToString for OwsqlError {
    fn to_string(&self) -> String {
        match self {
            OwsqlError::Code(i) =>    i.to_string(),
            OwsqlError::Message(s) => s.to_string(),
            OwsqlError::Err(()) =>    String::new(),
        }
    }
}

impl From::<()> for OwsqlError {
    fn from(_: ()) -> Self {
        OwsqlError::Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owsql_error() {
        assert_eq!(OwsqlError::Code(0).to_string(), "0");
        assert_eq!(OwsqlError::Message("test".to_string()).to_string(), "test");
        assert_eq!(OwsqlError::Err(()).to_string(), "");
        assert_eq!(OwsqlError::from(()), OwsqlError::Err(()));
    }
}

