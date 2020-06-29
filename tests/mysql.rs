#[cfg(feature = "mysql")]
mod mysql {

    fn stmt() -> &'static str {
        r#"CREATE TEMPORARY TABLE users (name TEXT, age INTEGER);
           INSERT INTO users (name, age) VALUES ('Alice', 42);
           INSERT INTO users (name, age) VALUES ('Bob', 69);
           INSERT INTO users (name, age) VALUES ('Carol', 50);"#
    }

    #[test]
    fn open() {
        let _conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
    }

    #[test]
    fn execute() {
        let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();
    }

    #[test]
    #[should_panic = "exec error"]
    fn execute_should_error() {
        let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.execute(stmt()).unwrap();
    }

    #[test]
    fn iterate() {
        let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        let expects = ["Alice", "Bob", "Carol"];
        conn.execute(&conn.ow(stmt())).unwrap();

        let sql = conn.ow("SELECT name FROM users;");

        conn.iterate(&sql, |pairs| {
            for (i, (_, value)) in pairs.iter().enumerate() {
                assert_eq!(value.as_ref().unwrap(), expects[i]);
            }
            true
        }).unwrap();
    }
}
