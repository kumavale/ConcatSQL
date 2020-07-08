#[cfg(feature = "postgres")]
#[cfg(debug_assertions)]
mod postgres {
    fn stmt() -> &'static str {
        r#"CREATE TEMPORARY TABLE users (name TEXT, age INTEGER);
           INSERT INTO users (name, age) VALUES ('Alice', 42);
           INSERT INTO users (name, age) VALUES ('Bob', 69);
           INSERT INTO users (name, age) VALUES ('Carol', 50);"#
    }

    #[test]
    fn open() {
        let _conn = owsql::postgres::open("host=localhost user=postgres password=postgres").unwrap();
    }

    #[test]
    fn execute() {
        let conn = owsql::postgres::open("host=localhost user=postgres password=postgres").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();
    }

}
