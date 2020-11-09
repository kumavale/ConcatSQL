#[cfg(feature = "postgres")]
#[cfg(debug_assertions)]
mod postgres {
    use concatsql::prelude::*;
    use concatsql::{Error, ErrorLevel};

    macro_rules! err {
        () => { Err(Error::AnyError) };
        ($msg:expr) => { Err(Error::Message($msg.to_string())) };
    }

    fn prepare() -> concatsql::Connection {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let stmt = prepare!(stmt());
        conn.execute(&stmt).unwrap();
        conn
    }

    fn stmt() -> &'static str {
        r#"CREATE TEMPORARY TABLE users (name TEXT, age INTEGER);
           INSERT INTO users (name, age) VALUES ('Alice', 42);
           INSERT INTO users (name, age) VALUES ('Bob', 69);
           INSERT INTO users (name, age) VALUES ('Carol', 50);"#
    }

    #[test]
    fn open() {
        let _conn = concatsql::postgres::open("host=localhost user=postgres password=postgres").unwrap();
        let _conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
    }

    #[test]
    fn execute() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let stmt = prepare!(stmt());
        conn.execute(&stmt).unwrap();
    }

    #[test]
    #[should_panic = "exec error"]
    fn execute_should_error() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.execute(stmt().to_wrapstring()).unwrap();
    }

    #[test]
    fn iterate() {
        let conn = prepare();
        let expects = ["Alice", "Bob", "Carol"];
        let sql = prepare!("SELECT name FROM users;");

        let mut i = 0;
        conn.iterate(&sql, |pairs| {
            for (_, value) in pairs {
                assert_eq!(*value.as_ref().unwrap(), expects[i]);
                i += 1;
            }
            true
        }).unwrap();
    }

    #[test]
    #[should_panic = "exec error"] // TODO support multiple statement
    fn iterate_2sets() {
        let conn = prepare();
        let sql = prepare!("SELECT name FROM users; SELECT name FROM users;");

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn iterate_or() {
        let conn = prepare();
        let expects = ["Alice", "Bob"];
        let age = "50";
        let sql = prepare!("SELECT name FROM users WHERE ") +
            &prepare!("age < ") + age + &prepare!(" OR ") + age + &prepare!(" < age");

        let mut i = 0;
        conn.iterate(&sql, |pairs| {
            for (_, value) in pairs {
                assert_eq!(*value.as_ref().unwrap(), expects[i]);
                i += 1;
            }
            true
        }).unwrap();
    }

    #[test]
    fn rows() {
        let conn = prepare();
        let expects = [("Carol", 50), ("Bob", 69), ("Alice", 42),];
        let sql = prepare!("SELECT * FROM users;");

        let rows = conn.rows(&sql).unwrap();
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.get("name").unwrap(), expects[i].0);
            assert_eq!(row.get("age").unwrap(),  expects[i].1.to_string());
        }
    }

    #[test]
    fn rows_foreach() {
        let conn = prepare();
        let expects = [("Carol", 50), ("Bob", 69), ("Alice", 42),];

        conn.rows(&prepare!("SELECT * FROM users;")).unwrap().iter().enumerate().for_each(|(i, row)| {
            assert_eq!(row.get("name").unwrap(), expects[i].0);
            assert_eq!(row.get("age").unwrap(),  expects[i].1.to_string());
        });
    }

    #[test]
    fn double_quotaion_inside_double_quote() {
        assert_eq!(
            r#"".ow(""inside str"") -> String""#.actual_sql(),
            r#"'".ow(""inside str"") -> String"'"#
        );
        assert_eq!(
            r#"".ow("inside str") -> String""#.actual_sql(),
            r#"'".ow("inside str") -> String"'"#
        );
    }

    #[test]
    fn double_quotaion_inside_sigle_quote() {
        assert_eq!(
            r#""I'm Alice""#.actual_sql(),
            r#"'"I''m Alice"'"#
        );
        assert_eq!(
            r#""I''m Alice""#.actual_sql(),
            r#"'"I''''m Alice"'"#
        );
    }

    #[test]
    fn single_quotaion_inside_double_quote() {
        assert_eq!(
            r#"'.ow("inside str") -> String'"#.actual_sql(),
            r#"'''.ow("inside str") -> String'''"#
        );
    }

    #[test]
    fn single_quotaion_inside_sigle_quote() {
        assert_eq!(
            "'I''m Alice'".actual_sql(),
            r#"'''I''''m Alice'''"#
        );
    }

    #[test]
    fn non_quotaion_inside_sigle_quote() {
        assert_eq!(
            "foo'bar'foo".actual_sql(),
            r#"'foo''bar''foo'"#
        );
    }

    #[test]
    fn non_quotaion_inside_double_quote() {
        assert_eq!(
            "foo\"bar\"foo".actual_sql(),
            r#"'foo"bar"foo'"#
        );
    }

    #[test]
    fn start_with_quotation_and_end_with_anything_else() {
        let conn = prepare();
        let name = "'Alice'; DROP TABLE users; --";
        let sql = prepare!("select age from users where name = ") + name + &prepare!("");
        assert_eq!(
            name.actual_sql(),
            r#"'''Alice''; DROP TABLE users; --'"#
        );
        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn whitespace() {
        let conn = prepare();
        let sql = prepare!("select\n*\rfrom\nusers;");

        conn.iterate(&sql, |_| { true }).unwrap();
    }

    #[test]
    fn sqli_eq_nonquote() {
        let conn = prepare();
        let name = "Alice' or '1'='1";
        let sql = prepare!("select age from users where name =") + name + &prepare!(";");
        // "select age from users where name = 'Alice'' or ''1''=''1';"

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn sanitizing() {
        let conn = prepare();
        let name = r#"<script>alert("&1");</script>"#;
        let sql = prepare!("INSERT INTO users VALUES(") + name + &prepare!(", 12345);");

        conn.execute(&sql).unwrap();

        conn.rows(prepare!("SELECT name FROM users WHERE age = 12345;")).unwrap().iter() .all(|row| {
            assert_eq!(
                concatsql::html_special_chars(row.get("name").unwrap()),
                "&lt;script&gt;alert(&quot;&amp;1&quot;);&lt;/script&gt;"
            );
            true
        });
    }

    #[test]
    fn error_level() {
        let mut conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.error_level(ErrorLevel::AlwaysOk);
        conn.error_level(ErrorLevel::Release);
        conn.error_level(ErrorLevel::Develop);
        conn.error_level(ErrorLevel::Debug);
    }

    #[test]
    #[allow(non_snake_case)]
    fn error_level_AlwaysOk() {
        let mut conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.error_level(ErrorLevel::AlwaysOk);
        let invalid_sql = "INVALID SQL".to_wrapstring();
        let endless = "'endless".to_wrapstring();

        assert_eq!(conn.execute(&invalid_sql),                      Ok(()));
        assert_eq!(conn.execute(&endless),                          Ok(()));
        assert_eq!(conn.iterate(&invalid_sql,  |_| unreachable!()), Ok(()));
        assert_eq!(conn.iterate(&endless,      |_| unreachable!()), Ok(()));
        assert_eq!(conn.rows(&invalid_sql),                         Ok(vec![]));
        assert_eq!(conn.rows(&endless),                             Ok(vec![]));
    }

    #[test]
    fn error_level_release() {
        let mut conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.error_level(ErrorLevel::Release);
        let invalid_sql = "INVALID SQL".to_wrapstring();
        let endless = "'endless".to_wrapstring();

        assert_eq!(conn.execute(&invalid_sql),                      err!());
        assert_eq!(conn.execute(&endless),                          err!());
        assert_eq!(conn.iterate(&invalid_sql,  |_| unreachable!()), err!());
        assert_eq!(conn.iterate(&endless,      |_| unreachable!()), err!());
        assert_eq!(conn.rows(&invalid_sql),                         err!());
        assert_eq!(conn.rows(&endless),                             err!());
    }

    #[test]
    fn error_level_develop() {
        let mut conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.error_level(ErrorLevel::Develop);
        let invalid_sql = "INVALID SQL".to_wrapstring();
        let endless = "'endless".to_wrapstring();

        assert_eq!(conn.execute(&invalid_sql),                      err!("exec error"));
        assert_eq!(conn.execute(&endless),                          err!("exec error"));
        assert_eq!(conn.iterate(&invalid_sql,  |_| unreachable!()), err!("exec error"));
        assert_eq!(conn.iterate(&endless,      |_| unreachable!()), err!("exec error"));
        assert_eq!(conn.rows(&invalid_sql),                         err!("exec error"));
        assert_eq!(conn.rows(&endless),                             err!("exec error"));
    }

    #[test]
    fn error_level_debug() {
        let mut conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.error_level(ErrorLevel::Debug);
        let invalid_sql = "INVALID SQL".to_wrapstring();
        let endless = "'endless".to_wrapstring();

        assert_eq!(conn.execute(&invalid_sql),
            err!("exec error: db error: ERROR: \"\'INVALID SQL\'\"またはその近辺で構文エラー"));
        assert_eq!(conn.execute(&endless),
            err!("exec error: db error: ERROR: \"\'\'\'endless\'\"またはその近辺で構文エラー"));
        assert_eq!(conn.iterate(&invalid_sql, |_| unreachable!()),
            err!("exec error: db error: ERROR: \"\'INVALID SQL\'\"またはその近辺で構文エラー"));
        assert_eq!(conn.iterate(&endless,    |_| unreachable!()),
            err!("exec error: db error: ERROR: \"\'\'\'endless\'\"またはその近辺で構文エラー"));
        assert_eq!(conn.rows(&invalid_sql),
            err!("exec error: db error: ERROR: \"\'INVALID SQL\'\"またはその近辺で構文エラー"));
        assert_eq!(conn.rows(&endless),
            err!("exec error: db error: ERROR: \"\'\'\'endless\'\"またはその近辺で構文エラー"));
    }

    #[test]
    fn integer() {
        let conn = prepare();
        let age = 50;
        let sql = prepare!("select name from users where age < ") + int!(age).unwrap();

        for row in conn.rows(&sql).unwrap().iter() {
            assert_eq!(row.get("name").unwrap(), "Alice");
        }
    }

    #[test]
    fn ow_into_execute() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.execute(prepare!("SELECT ") + int!(1).unwrap()).unwrap();
    }

    #[test]
    fn ow_into_iterate() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.iterate(prepare!("SELECT ") + int!(1).unwrap(), |_| true ).unwrap();
    }

    #[test]
    fn ow_into_rows() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        for row in conn.rows(prepare!("SELECT ") + int!(1).unwrap()).unwrap().iter() {
            assert_eq!(row.get("?column?").unwrap(), "1");
        }
    }
}
