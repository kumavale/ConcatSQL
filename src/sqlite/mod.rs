//! Interface to [SQLite](https://www.sqlite.org) of OverwriteSQL.

use std::path::Path;

#[macro_use]
mod parser;
mod connection;
mod token;

pub use self::connection::Connection;

/// Open a read-write connection to a new or existing database.
pub fn open<T: AsRef<Path>>(path: T) -> Result<Connection, String> {
    Connection::open(path)
}


#[cfg(test)]
mod tests {

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
}
