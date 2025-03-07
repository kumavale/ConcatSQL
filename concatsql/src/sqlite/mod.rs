//! Interface to [SQLite](https://www.sqlite.org) of ConcatSQL.

use crate::connection::Connection;
use crate::Result;
use std::path::Path;

pub(crate) mod connection;

/// Open a read-write connection to a new or existing database.
///
/// The default mode is serialized [threading mode](https://www.sqlite.org/threadsafe.html).
///
/// # Examples
///
/// ```
/// # fn main() {
/// #     test().ok();
/// # }
/// # fn test() -> Result<(), concatsql::Error> {
/// // Open a new connection to an in-memory.
/// let conn = concatsql::sqlite::open(":memory:")?;
/// // Open a new connection from path of literal.
/// let conn = concatsql::sqlite::open("/path/to/db")?;
/// // Open a new connection from std::path::Path.
/// let path = std::path::Path::new("/path/to/db");
/// let conn = concatsql::sqlite::open(path)?;
/// # Ok(())
/// # }
/// ```
#[inline]
pub fn open<T: AsRef<Path>>(path: T) -> Result<Connection> {
    connection::open(
        path,
        sqlite3_sys::SQLITE_OPEN_CREATE | sqlite3_sys::SQLITE_OPEN_READWRITE,
    )
}

/// Open a readonly connection to a new or existing database.
#[inline]
pub fn open_readonly<T: AsRef<Path>>(path: T) -> Result<Connection> {
    connection::open(path, sqlite3_sys::SQLITE_OPEN_READONLY)
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
    use crate as concatsql;
    use concatsql::prelude::*;
    use temporary::Folder;

    #[test]
    fn sqlite_open() {
        let dir = Folder::new("sqlite").unwrap();
        let path = dir.path().join("test.db");
        crate::sqlite::open(":memory:").unwrap();
        crate::sqlite::open(path).unwrap();
    }

    #[test]
    fn sqlite_open_readonly() {
        crate::sqlite::open_readonly(":memory:").unwrap();

        let dir = Folder::new("sqlite").unwrap();
        let path = dir.path().join("test.db");
        {
            let conn = crate::sqlite::open(&path).unwrap();
            conn.execute(query!("CREATE TABLE users(id INTEGER, name TEXT);"))
                .unwrap();
        }
        crate::sqlite::open_readonly(path).unwrap();
    }

    #[test]
    #[cfg(debug_assertions)]
    fn should_readonly() {
        use crate::error::*;
        let dir = Folder::new("sqlite").unwrap();
        let path = dir.path().join("test.db");
        {
            let conn = crate::sqlite::open(&path).unwrap();
            conn.execute(query!("CREATE TABLE users(id INTEGER, name TEXT);"))
                .unwrap();
        }
        let conn = crate::sqlite::open_readonly(path).unwrap();
        conn.error_level(ErrorLevel::Debug);
        assert_eq!(
            conn.execute(query!("INSERT INTO users VALUES(42, 'Alice');")),
            Err(Error::Message(
                "exec error: attempt to write a readonly database".to_string()
            ))
        );
    }

    #[test]
    #[should_panic = "failed to connect"]
    fn sqlite_open_failed_path() {
        use std::path::Path;
        let _conn = crate::sqlite::open(Path::new("/path/to/db")).unwrap();
    }
    #[test]
    #[should_panic = "failed to connect"]
    fn sqlite_open_failed_str() {
        let _conn = crate::sqlite::open("/path/to/db").unwrap();
    }

    #[test]
    fn version() {
        crate::sqlite::version();
    }
}
