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
        let _conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
    }

    #[test]
    fn execute() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();
    }

    #[test]
    #[should_panic = "exec error"]
    fn execute_should_error() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.execute(stmt()).unwrap();
    }

    #[test]
    fn iterate() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let expects = ["Alice", "Bob", "Carol"];
        conn.execute(&conn.ow(stmt())).unwrap();

        let sql = conn.ow("SELECT name FROM users;");

        let mut i = 0;
        conn.iterate(&sql, |pairs| {
            for (_, value) in pairs {
                assert_eq!(value.as_ref().unwrap(), expects[i]);
                i += 1;
            }
            true
        }).unwrap();
    }

    #[test]
    #[should_panic = "exec error"] // TODO support multiple statement
    fn iterate_2sets() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.execute(&conn.ow(stmt())).unwrap();

        let sql = conn.ow("SELECT name FROM users; SELECT name FROM users;");

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn iterate_or() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let expects = ["Alice", "Bob"];
        conn.execute(&conn.ow(stmt())).unwrap();

        let age = "50";
        let sql = conn.ow("SELECT name FROM users WHERE") +
            &conn.ow("age <") + age + &conn.ow("OR") + age + &conn.ow("< age");

        let mut i = 0;
        conn.iterate(&sql, |pairs| {
            for (_, value) in pairs {
                assert_eq!(value.as_ref().unwrap(), expects[i]);
                i += 1;
            }
            true
        }).unwrap();
    }
}
