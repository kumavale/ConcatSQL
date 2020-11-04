//! Interface to [PostgreSQL](https://www.postgresql.org/) of OverwriteSQL.

pub(crate) mod connection;

use crate::Result;
use crate::connection::Connection;
use crate::error::OwsqlErrorLevel;

/// Open a read-write connection to a new or existing database.
///
/// See the documentation for [Config](https://docs.rs/postgres/latest/postgres/config/struct.Config.html) for information about the connection syntax.
///
/// # Examples
///
/// ```rust
/// let params = "host=localhost user=postgres password=postgres";
/// let conn = owsql::postgres::open(&params).unwrap();
/// ```
#[inline]
pub fn open(params: &str) -> Result<Connection> {
    connection::open(&params)
}

/// Open a read-write connection to a new or existing database with OwsqlErrorLevel.
///
/// The default value is [OwsqlErrorLevel](./enum.OwsqlErrorLevel.html)::Develop for debug
/// builds and [OwsqlErrorLevel](./enum.OwsqlErrorLevel.html)::Release for release builds.
#[inline]
pub fn open_with_error_level(url: &str, error_level: OwsqlErrorLevel) -> Result<Connection> {
    let conn = connection::open(&url);
    conn.map(|conn| conn.error_level(error_level))
}

