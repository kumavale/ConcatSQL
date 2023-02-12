use std::fmt;

/// Enum listing possible errors from concatsql.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// The error message.
    Message(String),
    /// An any errors.
    AnyError,
    /// Return value when [get_into](./struct.Row.html#method.get_into) method fails.
    ParseError,
    /// Return value when [get_into](./struct.Row.html#method.get_into) method fails.
    ColumnNotFound,
}

/// Change the output error message.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ErrorLevel {
    /// No error message returned, always return Result::Ok(()).
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
    pub(crate) fn new<E1, E2>(error_level: &ErrorLevel, err_msg: E1, detail_msg: E2) -> Result<(), Error>
        where
            E1: ToString,
            E2: ToString,
    {
        match error_level {
            ErrorLevel::AlwaysOk => Ok(()),
            ErrorLevel::Release  => Err(Error::AnyError),
            ErrorLevel::Develop  => Err(Error::Message(err_msg.to_string())),
            #[cfg(debug_assertions)]
            ErrorLevel::Debug    => Err(Error::Message(err_msg.to_string() + ": " + &detail_msg.to_string())),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}",
            match self {
                Error::Message(s) =>     s.to_string(),
                Error::AnyError =>       String::from("AnyError"),
                Error::ParseError =>     String::from("ParseError"),
                Error::ColumnNotFound => String::from("ColumnNotFound"),
            }
        )
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self)
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

    #[test]
    #[cfg(feature = "sqlite")]
    fn error_level() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        conn.error_level(ErrorLevel::Develop);
        for _ in &conn.rows("SELECT 1").unwrap() {
            conn.error_level(ErrorLevel::Develop);
        }
        conn.error_level(ErrorLevel::Develop);
        conn.execute({
            conn.error_level(ErrorLevel::Develop);
            "SELECT 1"
        }).unwrap();
        conn.error_level({
            conn.execute("SELECT 1").unwrap();
            ErrorLevel::Develop
        });
    }
}

