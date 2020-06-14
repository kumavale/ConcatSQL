use std::path::Path;

mod connection;

use self::connection::Connection;

pub fn open<T: AsRef<Path>>(path: T) -> Result<Connection, String> {
    Connection::open(path)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_connect() {
        let _conn = crate::sqlite::open(":memory:").unwrap();
    }

    #[test]
    fn sqlite_execute() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        let stmt = r#"
            CREATE TABLE users (name TEXT, age INTEGER);
            INSERT INTO users (name, age) VALUES ('Alice', 42);"#;
        conn.execute(&stmt).unwrap();
    }
}
