
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
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();
    }

    #[test]
    fn iterate() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        let expects = ["Alice", "Bob", "Carol"];

        conn.execute(&stmt).unwrap();

        let mut i = 0;
        let query = conn.ow("SELECT name FROM users;");

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
        let stmt = conn.ow(stmt());
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

    //_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/
    /*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*
    /* TODO  TODO  TODO  TODO  TODO  TODO  TODO  TODO  TODO  TODO  TODO  TODO */
    */*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/
    //_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/
    #[test]
    #[ignore]
    #[should_panic = "exec error"]
    fn iterate_or_failed() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        let expects = ["Alice", "Bob", "unreachable"];
        //let expects = ["Bob", "Carol", "unreachable"];

        conn.execute(&stmt).unwrap();

        let mut i = 0;
        let age = "50 or 1=1; --";
        //let query = conn.ow("SELECT") + "name" +
        //    &conn.ow("FROM") + "users" +
        //    &conn.ow("WHERE") + "age" + &conn.ow("<") + "50" + or + "50" + &conn.ow("<") + "age;";
        let query = conn.ow(r#"SELECT name FROM users WHERE"#) +
            //&conn.ow("age") + &conn.ow("<") + age + &conn.ow("OR") + age + &conn.ow("<") + &conn.ow("age");
            &conn.ow("age") + &conn.ow("<") + age + &conn.ow("OR") + &conn.ow("50") + &conn.ow("<") + &conn.ow("age");
            //&conn.ow("age") + &conn.ow("<") + &conn.int(&age) + &conn.ow("OR") + &conn.ow("50") + &conn.ow("<") + &conn.ow("age");
        //let query = conn.ow("SELECT name FROM users WHERE age < '50' < age;");
        //let query = conn.ow("SELECT name FROM users WHERE age < ?");
        //let query = "select * from users;";
        //"SELECT name FROM users WHERE age < ? OR 50 < age;"
        //                                    ^
        //                            "'50 or 1=1;--'"
        // "INTEGER < TEXT" <= always TRUE
        // "INTEGER > TEXT" <= always FALSE

        conn.iterate(&query, |pairs| {  // error
            for &(_, value) in pairs.iter() {
                assert_eq!(value.unwrap(), expects[i]);
            }
            i += 1;
            true
        }).unwrap();
        assert_eq!(i, 2);
    }
}
