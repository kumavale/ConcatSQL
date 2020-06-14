use std::path::Path;

mod connection;

use self::connection::Connection;

pub fn open<T: AsRef<Path>>(path: T) -> Result<Connection, String> {
    Connection::open(path)
}
