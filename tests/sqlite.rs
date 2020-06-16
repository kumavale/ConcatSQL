
#[cfg(feature = "sqlite")]
mod sqlite {
    extern crate owsql;

    fn stmt() -> &'static str {
        r#"CREATE TABLE users (name TEXT, age INTEGER);
           INSERT INTO users (name, age) VALUES ('Alice', 42);
           INSERT INTO users (name, age) VALUES ('Bob', 69);
           INSERT INTO users (name, age) VALUES ('Carol', 50);"#
    }

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
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = stmt();
        let expects = ["Alice", "Bob", "Carol"];

        conn.execute(&stmt).unwrap();

        let mut i = 0;
        let query = conn.ow("SELECT") + "name" + &conn.ow("FROM") + "users;";

        conn.iterate(&query, |pairs| {
            for &(_, value) in pairs.iter() {
                assert_eq!(value.unwrap(), expects[i]);
            }
            i += 1;
            true
        }).unwrap();
    }

    #[test]
    fn iterate_or() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = stmt();
        let expects = ["Alice", "Bob"];

        conn.execute(&stmt).unwrap();

        let mut i = 0;
        let age = "50";
        //let age = "'50'";
        //let age = "50 or 1=1; --";
        let query = conn.ow("SELECT name FROM users WHERE") +
            &conn.ow("age <") + age + &conn.ow("OR") + age + &conn.ow("< age");

        conn.iterate(&query, |pairs| {
            for &(_, value) in pairs.iter() {
                assert_eq!(value.unwrap(), expects[i]);
            }
            i += 1;
            true
        }).unwrap();
    }

    #[test]
    #[should_panic = "invalid syntax"]
    fn iterate_or_failed() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = stmt();
        let expects = ["Alice", "Bob"];

        conn.execute(&stmt).unwrap();

        let mut i = 0;
        let or = " or ";
        let query = conn.ow("SELECT") + "name" +
            &conn.ow("FROM") + "users" +
            &conn.ow("WHERE") + "age" + &conn.ow("<") + "50" + or + "50" + &conn.ow("<") + "age;";
        //"SELECT name FROM users WHERE age < '50 or 50' < age;"

        conn.iterate(&query, |pairs| {  // error
            for &(_, value) in pairs.iter() {
                assert_eq!(value.unwrap(), expects[i]);
            }
            i += 1;
            true
        }).unwrap();
    }
}
