
#[cfg(feature = "sqlite")]
mod sqlite {
    use owsql::params;

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
        let sql = conn.ow("SELECT name FROM users;");

        conn.iterate(&sql, |pairs| {
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
        let sql = conn.ow("SELECT name FROM users WHERE") +
            &conn.ow("age <") + age + &conn.ow("OR") + age + &conn.ow("< age");

        conn.iterate(&sql, |pairs| {
            for &(_, value) in pairs.iter() {
                assert_eq!(value.unwrap(), expects[i]);
            }
            i += 1;
            true
        }).unwrap();
    }

    #[test]
    fn double_quotaion_inside_double_quote() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();


        let name = r#"".ow(""inside str"") -> String""#;
        let sql = conn.ow("select age from users where name = ") + name;

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn double_quotaion_inside_sigle_quote() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = r#""I'm Alice""#;
        let sql = conn.ow("select age from users where name = ") + name;

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn single_quotaion_inside_double_quote() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = r#"'.ow("inside str") -> String'"#;
        let sql = conn.ow("select age from users where name = ") + name;

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn single_quotaion_inside_sigle_quote() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "'I''m Alice'";
        let sql = conn.ow("select age from users where name = ") + name;

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn non_quotaion_inside_sigle_quote() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "foo'bar'foo";
        let sql = conn.ow("select age from users where name = ") + name;

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn non_quotaion_inside_double_quote() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "foo\"bar\"foo";
        let sql = conn.ow("select age from users where name = ") + name;

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn non_quotaion_inside_double_quote_after_owstring() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "foo\"bar\"foo";
        let sql = conn.ow("select age from users where name = ") + name + &conn.ow("");

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn whitespace() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let sql = conn.ow("select\n*\rfrom\nusers;");

        conn.iterate(&sql, |_| { true }).unwrap();
    }

    #[test]
    fn sqli_eq_nonquote() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "Alice' or '1'='1";
        let sql = conn.ow("select age from users where name =") + name + &conn.ow(";");
        // "select age from users where name = 'Alice'' or ''1''=''1';"

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn allowlist() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        conn.add_allowlist(params![ 30 ]);
        let age = 30;
        let sql = conn.ow("select age from users where age <") + &conn.allowlist(age) + &conn.ow(";");

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    mod should_panic {
        use owsql::params;
        use super::stmt;

        #[test]
        #[should_panic = "exec error"]
        fn literal() {
            let mut conn = owsql::sqlite::open(":memory:").unwrap();
            let stmt = conn.ow(stmt());

            conn.execute(&stmt).unwrap();

            let sql = "select * from users;";

            conn.iterate(&sql, |_| { true }).unwrap();
        }

        #[test]
        #[should_panic = "endless"]
        fn endless_string() {
            let mut conn = owsql::sqlite::open(":memory:").unwrap();
            let stmt = conn.ow(stmt());
            conn.execute(&stmt).unwrap();

            let name = "'endless";
            let sql = conn.ow("select age from users where name =") + name + &conn.ow(";");

            conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
        }

        #[test]
        #[should_panic = "invalid literal"]
        fn sqli_eq_quote() {
            let mut conn = owsql::sqlite::open(":memory:").unwrap();
            let stmt = conn.ow(stmt());
            conn.execute(&stmt).unwrap();

            let name = "OR TRUE; DROP TABLE users; --";
            let sql = conn.ow("select age from users where name = '") + name + &conn.ow("';");

            conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
        }

        #[test]
        #[should_panic = "deny value"]
        fn allowlist_deny_value() {
            let mut conn = owsql::sqlite::open(":memory:").unwrap();
            let stmt = conn.ow(stmt());
            conn.execute(&stmt).unwrap();

            conn.add_allowlist(params!["Alice", "Bob"]);
            let name = "Alice OR 1=1; --";
            let sql = conn.ow("select age from users where name =") + &conn.allowlist(name) + &conn.ow(";");

            conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
        }
    }
}
