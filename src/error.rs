
/// Enum listing possible errors from concatsql.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Error {
    /// The error message.
    Message(String),
    /// An any errors.
    AnyError,
}

/// Change the output error message.
#[derive(Debug, PartialEq)]
pub enum ErrorLevel {
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

impl Default for ErrorLevel {
    fn default() -> Self {
        if cfg!(debug_assertions) {
            ErrorLevel::Develop
        } else {
            ErrorLevel::Release
        }
    }
}

impl Error {
    #[allow(unused_variables)]
    pub(crate) fn new(error_level: &ErrorLevel, err_msg: &str, detail_msg: &str) -> Result<(), Error> {
        match error_level {
            ErrorLevel::AlwaysOk => Ok(()),
            ErrorLevel::Release  => Err(Error::AnyError),
            ErrorLevel::Develop  => Err(Error::Message(err_msg.to_string())),
            #[cfg(debug_assertions)]
            ErrorLevel::Debug    => Err(Error::Message(format!("{}: {}", err_msg, detail_msg))),
        }
    }
}

impl std::string::ToString for Error {
    fn to_string(&self) -> String {
        match self {
            Error::Message(s) => s.to_string(),
            Error::AnyError =>   String::from("AnyError"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(debug_assertions)]
    fn errors() {
        assert_eq!(ErrorLevel::default(), ErrorLevel::Develop);
        assert_eq!(Error::Message("test".to_string()).to_string(), "test");
        assert_eq!(
            Error::new(&ErrorLevel::AlwaysOk, "test", "test"),
            Ok(()));
        assert_eq!(
            Error::new(&ErrorLevel::Release,  "test", "test"),
            Err(Error::AnyError));
        assert_eq!(
            Error::new(&ErrorLevel::Develop,  "test", "test"),
            Err(Error::Message("test".into())));
        assert_eq!(
            Error::new(&ErrorLevel::Debug,    "test", "test"),
            Err(Error::Message("test: test".into())));
    }
}

