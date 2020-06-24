//! `feature = "sqlite"` Interface to [SQLite](https://www.sqlite.org) of OverwriteSQL.

use std::path::Path;
use crate::Result;

#[macro_use]
mod parser;
mod connection;
mod token;
mod row;

pub use self::row::Row;
pub use self::connection::Connection;

/// Open a read-write connection to a new or existing database.
///
/// The default mode is serialized [threading mode](https://www.sqlite.org/threadsafe.html).
///
/// # Examples
///
/// ```should_panic
/// // Open a new connection to an in-memory.
/// let conn = owsql::sqlite::open(":memory:").unwrap();
/// // Open a new connection from path of literal.
/// let conn = owsql::sqlite::open("/path/to/db").unwrap();
/// // Open a new connection from std::path::Path.
/// let path = std::path::Path::new("/path/to/db");
/// let conn = owsql::sqlite::open(path).unwrap();
/// ```
#[inline]
pub fn open<T: AsRef<Path>>(path: T) -> Result<Connection> {
    Connection::open(path,
        sqlite3_sys::SQLITE_OPEN_CREATE |
        sqlite3_sys::SQLITE_OPEN_READWRITE
    )
}

/// Open a readonly connection to a new or existing database.
#[inline]
pub fn open_readonly<T: AsRef<Path>>(path: T) -> Result<Connection> {
    Connection::open(path,
        sqlite3_sys::SQLITE_OPEN_CREATE |
        sqlite3_sys::SQLITE_OPEN_READWRITE |
        sqlite3_sys::SQLITE_OPEN_READONLY
    )
}

/// Return the version number of SQLite.
///
/// For instance, the version `3.32.2` corresponds to the integer `3032002`.
#[inline]
pub fn version() -> usize {
    unsafe { sqlite3_sys::sqlite3_libversion_number() as usize }
}


#[cfg(test)]
mod tests {
    use crate::*;
    use crate::value::Value;

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

    #[test]
    fn version() {
        crate::sqlite::version();
    }
}
