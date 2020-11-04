//! Interface to [MySQL](https://www.mysql.com/) of OverwriteSQL.

pub(crate) mod connection;

use crate::Result;
use crate::connection::Connection;

/// Open a read-write connection to a new or existing database.
///
/// URL schema must be mysql. Host, port and credentials, as well as query parameters, should be given in
/// accordance with the RFC 3986.
///
/// # Examples
///
/// ```rust
/// let url = "mysql://user:password@localhost:3306/db_name";
/// let conn = owsql::mysql::open(&url).unwrap();
/// ```
#[inline]
pub fn open(url: &str) -> Result<Connection> {
    connection::open(&url)
}

