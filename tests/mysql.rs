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
    #[should_panic("exec error")]
    fn execute_should_error() {
        let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.execute(stmt()).unwrap();
    }
}
