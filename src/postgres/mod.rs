//! Interface to [PostgreSQL](https://www.postgresql.org/) of OverwriteSQL.

pub(crate) mod connection;

use crate::Result;

pub use self::connection::PostgreSQLConnection;

/// Open a read-write connection to a new or existing database.
///
/// # Examples
///
/// ```rust
/// let params = "host=localhost user=postgres password=postgres";
/// let conn = owsql::postgres::open(&params).unwrap();
/// ```
#[inline]
pub fn open(params: &str) -> Result<PostgreSQLConnection> {
    PostgreSQLConnection::open(&params)
}

