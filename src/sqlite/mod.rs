//! `feature = "sqlite"` Interface to [SQLite](https://www.sqlite.org) of OverwriteSQL.

use std::path::Path;
use crate::Result;

#[macro_use]
mod parser;
mod connection;
mod token;
pub mod value;

/// Output type of params macro.
type Params = Vec<value::Value>;

pub use self::connection::Connection;

/// Open a read-write connection to a new or existing database.
pub fn open<T: AsRef<Path>>(path: T) -> Result<Connection> {
    Connection::open(path)
}


#[cfg(test)]
mod tests {
    use crate::*;
    use crate::sqlite::value::Value;

    #[test]
    fn sqlite_open() {
        let _conn = crate::sqlite::open(":memory:").unwrap();
        #[cfg(unix)]
        let _conn = crate::sqlite::open("/tmp/tmp.db").unwrap();
    }

    #[test]
    #[should_panic = "failed to connect"]
    fn sqlite_open_failed() {
        use std::path::Path;
        let _conn = crate::sqlite::open(Path::new("/path/to/db")).unwrap();
    }

    #[test]
    #[allow(clippy::blacklisted_name)]
    fn params_macro() {
        let foo = 42;
        let bar = "bar";

        assert_eq!(vec![Value::Int(42)], params![ 42 ]);
        assert_eq!(vec![Value::String(String::from("bar"))], params![ "bar" ]);
        assert_eq!(vec![Value::Int(42)], params![ foo ]);
        assert_eq!(vec![Value::String(String::from("bar"))], params![ bar ]);
        assert_eq!(
            vec![Value::Int(42), Value::String(String::from("bar")),],
            params![ foo, bar ]
        );
    }
}
