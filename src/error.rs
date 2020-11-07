
/// Enum listing possible errors from concatsql.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ConcatsqlError {
    /// The error message.
    Message(String),
    /// An any errors.
    AnyError,
}

/// Change the output error message.
#[derive(Debug, PartialEq)]
pub enum ConcatsqlErrorLevel {
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

impl Default for ConcatsqlErrorLevel {
    fn default() -> Self {
        if cfg!(debug_assertions) {
            ConcatsqlErrorLevel::Develop
        } else {
            ConcatsqlErrorLevel::Release
        }
    }
}

impl ConcatsqlError {
    #[allow(unused_variables)]
    pub(crate) fn new(error_level: &ConcatsqlErrorLevel, err_msg: &str, detail_msg: &str) -> Result<(), ConcatsqlError> {
        match error_level {
            ConcatsqlErrorLevel::AlwaysOk => Ok(()),
            ConcatsqlErrorLevel::Release  => Err(ConcatsqlError::AnyError),
            ConcatsqlErrorLevel::Develop  => Err(ConcatsqlError::Message(err_msg.to_string())),
            #[cfg(debug_assertions)]
            ConcatsqlErrorLevel::Debug    => Err(ConcatsqlError::Message(format!("{}: {}", err_msg, detail_msg))),
        }
    }
}

impl std::string::ToString for ConcatsqlError {
    fn to_string(&self) -> String {
        match self {
            ConcatsqlError::Message(s) => s.to_string(),
            ConcatsqlError::AnyError =>   String::from("AnyError"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(debug_assertions)]
    fn owsql_error() {
        assert_eq!(ConcatsqlErrorLevel::default(), ConcatsqlErrorLevel::Develop);
        assert_eq!(ConcatsqlError::Message("test".to_string()).to_string(), "test");
        assert_eq!(
            ConcatsqlError::new(&ConcatsqlErrorLevel::AlwaysOk, "test", "test"),
            Ok(()));
        assert_eq!(
            ConcatsqlError::new(&ConcatsqlErrorLevel::Release,  "test", "test"),
            Err(ConcatsqlError::AnyError));
        assert_eq!(
            ConcatsqlError::new(&ConcatsqlErrorLevel::Develop,  "test", "test"),
            Err(ConcatsqlError::Message("test".into())));
        assert_eq!(
            ConcatsqlError::new(&ConcatsqlErrorLevel::Debug,    "test", "test"),
            Err(ConcatsqlError::Message("test: test".into())));
    }
}

