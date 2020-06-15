
#[cfg(feature = "sqlite")]
mod sqlite {
    extern crate owsql;

    #[test]
    fn open() {
        let _conn = owsql::sqlite::open(":memory:").unwrap();
    }

    #[test]
    fn execute() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = stmt();
        conn.execute(&stmt).unwrap();
    }

    #[test]
    fn iterate() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = stmt();
        let expects = ["Alice", "Bob", "Carol"];

        conn.execute(&stmt).unwrap();

        let mut i = 0;
        let query = "SELECT name FROM users;";
        conn.iterate(&query, |pairs| {
            for &(_, value) in pairs.iter() {
                assert_eq!(value.unwrap(), expects[i]);
            }
            i += 1;
            true
        }).unwrap();
    }

    fn stmt() -> &'static str {
        r#"CREATE TABLE users (name TEXT, age INTEGER);
           INSERT INTO users (name, age) VALUES ('Alice', 42);
           INSERT INTO users (name, age) VALUES ('Bob', 69);
           INSERT INTO users (name, age) VALUES ('Carol', 50);"#
    }
}
