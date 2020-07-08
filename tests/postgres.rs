#[cfg(feature = "postgres")]
#[cfg(debug_assertions)]
mod postgres {
    use owsql::params;

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

    #[test]
    fn rows() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
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
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let expects = [("Carol", 50), ("Bob", 69), ("Alice", 42),];
        conn.execute(&conn.ow(stmt())).unwrap();

        conn.rows(&conn.ow("SELECT * FROM users;")).unwrap().iter().enumerate().for_each(|(i, row)| {
            assert_eq!(row.get("name").unwrap(), expects[i].0);
            assert_eq!(row.get("age").unwrap(),  expects[i].1.to_string());
        });
    }

    #[test]
    fn double_quotaion_inside_double_quote() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = r#"".ow(""inside str"") -> String""#;  // expect: '".ow(""inside str"") -> String"'
        let sql = conn.ow("select age from users where name = ") + unsafe { &conn.ow_without_html_escape(&name) };
        conn.execute(&sql).unwrap();

        let name = r#"".ow("inside str") -> String""#;  // expect: '".ow("inside str") -> String"'
        let sql = conn.ow("select age from users where name = ") + unsafe { &conn.ow_without_html_escape(&name) };
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn double_quotaion_inside_sigle_quote() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = r#""I'm Alice""#; // expect: '"I''m Alice"'
        let sql = conn.ow("select age from users where name = ") + unsafe { &conn.ow_without_html_escape(&name) };
        conn.execute(&sql).unwrap();
        let name = r#""I''m Alice""#; // expect: '"I''''m Alice"'
        let sql = conn.ow("select age from users where name = ") + unsafe { &conn.ow_without_html_escape(&name) };
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn single_quotaion_inside_double_quote() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = r#"'.ow("inside str") -> String'"#; // expect: '''.ow("inside str") -> String'''
        let sql = conn.ow("select age from users where name = ") + name;
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn single_quotaion_inside_sigle_quote() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "'I''m Alice'"; // expect: '''I''''m Alice'''
        let sql = conn.ow("select age from users where name = ") + name;
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn non_quotaion_inside_sigle_quote() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "foo'bar'foo"; // expect: 'foo''bar''foo'
        let sql = conn.ow("select age from users where name = ") + name;
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn non_quotaion_inside_double_quote() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "foo\"bar\"foo"; // expect: 'foo\"bar\"foo'
        let sql = conn.ow("select age from users where name = ") + name;
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn non_quotaion_inside_double_quote_after_owstring() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "foo\"bar\"foo"; // expect: 'foo\"bar\"foo'
        let sql = conn.ow("select age from users where name = ") + name + &conn.ow("");
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn start_with_quotation_and_end_with_anything_else() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "'Alice'; DROP TABLE users; --"; // expect: '''Alice''); DROP TABLE users; --'
        let sql = conn.ow("select age from users where name = ") + name + &conn.ow("");

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn whitespace() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let sql = conn.ow("select\n*\rfrom\nusers;");

        conn.iterate(&sql, |_| { true }).unwrap();
    }

    #[test]
    fn sqli_eq_nonquote() {
        let conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "Alice' or '1'='1";
        let sql = conn.ow("select age from users where name =") + name + &conn.ow(";");
        // "select age from users where name = 'Alice'' or ''1''=''1';"

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn allowlist() {
        let mut conn = owsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        conn.add_allowlist(params![ 30 ]);
        let age = 30;
        let sql = conn.ow("select age from users where age <") + &conn.allowlist(age) + &conn.ow(";");

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }
}
