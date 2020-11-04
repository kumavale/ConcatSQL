
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

    #[cfg(debug_assertions)]
    /// Output more detailed messages during development.  
    /// &#x26a0;&#xfe0f; **Not available when Release build**  
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
    #[allow(unused_variables)]
    pub(crate) fn new(error_level: &OwsqlErrorLevel, err_msg: &str, detail_msg: &str) -> Result<(), OwsqlError> {
        match error_level {
            OwsqlErrorLevel::AlwaysOk => Ok(()),
            OwsqlErrorLevel::Release  => Err(OwsqlError::AnyError),
            OwsqlErrorLevel::Develop  => Err(OwsqlError::Message(err_msg.to_string())),
            #[cfg(debug_assertions)]
            OwsqlErrorLevel::Debug    => Err(OwsqlError::Message(format!("{}: {}", err_msg, detail_msg))),
        }
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
    #[cfg(debug_assertions)]
    fn owsql_error() {
        assert_eq!(OwsqlErrorLevel::default(), OwsqlErrorLevel::Develop);
        assert_eq!(OwsqlError::Message("test".to_string()).to_string(), "test");
        assert_eq!(
            OwsqlError::new(&OwsqlErrorLevel::AlwaysOk, "test", "test"),
            Ok(()));
        assert_eq!(
            OwsqlError::new(&OwsqlErrorLevel::Release,  "test", "test"),
            Err(OwsqlError::AnyError));
        assert_eq!(
            OwsqlError::new(&OwsqlErrorLevel::Develop,  "test", "test"),
            Err(OwsqlError::Message("test".into())));
        assert_eq!(
            OwsqlError::new(&OwsqlErrorLevel::Debug,    "test", "test"),
            Err(OwsqlError::Message("test: test".into())));
    }
}

