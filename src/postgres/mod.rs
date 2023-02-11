//! Interface to [PostgreSQL](https://www.postgresql.org/) of ConcatSQL.

pub(crate) mod connection;

use crate::Result;
use crate::connection::Connection;

/// Open a read-write connection to a new or existing database.
///
/// See the documentation for [Config](https://docs.rs/postgres/latest/postgres/config/struct.Config.html) for information about the connection syntax.
///
/// # Examples
///
/// ```rust
/// let params = "host=localhost user=postgres password=postgres";
/// let conn = concatsql::postgres::open(&params).unwrap();
/// ```
#[inline]
pub fn open(params: &str) -> Result<Connection> {
    connection::open(params)
}

