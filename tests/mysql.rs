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

    #[test]
    fn iterate_2sets() {
        let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        let expects = ["Alice", "Bob", "Carol", "Alice", "Bob", "Carol"];
        conn.execute(&conn.ow(stmt())).unwrap();

        let sql = conn.ow("SELECT name FROM users; SELECT name FROM users;");

        conn.iterate(&sql, |pairs| {
            for (i, (_, value)) in pairs.iter().enumerate() {
                assert_eq!(value.as_ref().unwrap(), expects[i]);
            }
            true
        }).unwrap();
    }

    #[test]
    fn iterate_or() {
        let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        let expects = ["Alice", "Bob"];
        conn.execute(&conn.ow(stmt())).unwrap();

        let age = "50";
        let sql = conn.ow("SELECT name FROM users WHERE") +
            &conn.ow("age <") + age + &conn.ow("OR") + age + &conn.ow("< age");

        conn.iterate(&sql, |pairs| {
            for (i, (_, value)) in pairs.iter().enumerate() {
                assert_eq!(value.as_ref().unwrap(), expects[i]);
            }
            true
        }).unwrap();
    }

    #[test]
    fn rows() {
        let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        let expects = [("Carol", 50), ("Bob", 69), ("Alice", 42),];
        conn.execute(&conn.ow(stmt())).unwrap();

        let sql = conn.ow("SELECT * FROM users;");

        let rows = conn.rows(&sql).unwrap();
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.get("name").unwrap(), expects[i].0);
            assert_eq!(row.get("age").unwrap(),  expects[i].1.to_string());
        }
    }

    #[test]
    fn rows_foreach() {
        let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        let expects = [("Carol", 50), ("Bob", 69), ("Alice", 42),];
        conn.execute(&conn.ow(stmt())).unwrap();

        conn.rows(&conn.ow("SELECT * FROM users;")).unwrap().iter().enumerate().for_each(|(i, row)| {
            assert_eq!(row.get("name").unwrap(), expects[i].0);
            assert_eq!(row.get("age").unwrap(),  expects[i].1.to_string());
        });
    }
}
