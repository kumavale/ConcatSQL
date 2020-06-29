//! Interface to [MySQL](https://www.mysql.com/) of OverwriteSQL.

mod connection;
mod parser;
mod token;

use crate::Result;

pub use self::connection::MysqlConnection;


#[inline]
pub fn open(url: &str) -> Result<MysqlConnection> {
    MysqlConnection::open(&url)
}
