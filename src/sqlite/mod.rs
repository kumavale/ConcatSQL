use std::path::Path;

mod connection;

use self::connection::Connection;

pub fn open<T: AsRef<Path>>(path: T) -> Result<Connection, String> {
    Connection::open(path)
}


#[cfg(test)]
mod tests {

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

    #[test]
    fn sqlite_iterate() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        let stmt = r#"
            CREATE TABLE users (name TEXT, age INTEGER);
            INSERT INTO users (name, age) VALUES ('Alice', 42);"#;
        let expect = ("name", "Alice");

        conn.execute(&stmt).unwrap();

        let query = "SELECT name FROM users;";
        conn.iterate(&query, |pairs| {
            for &(column, value) in pairs.iter() {
                assert_eq!(column,         expect.0);
                assert_eq!(value.unwrap(), expect.1);
            }
            true
        }).unwrap();
    }
}
