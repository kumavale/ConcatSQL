#[cfg(feature = "mysql")]
#[cfg(debug_assertions)]
mod mysql {
    use owsql::*;

    macro_rules! err {
        () => { Err(owsql::OwsqlError::AnyError) };
        ($msg:expr) => { Err(owsql::OwsqlError::Message($msg.to_string())) };
    }

    fn prepare() -> owsql::Connection {
        let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        let stmt = conn.ow(stmt());
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
        let conn = prepare();
        let expects = ["Alice", "Bob", "Carol"];
        let sql = conn.ow("SELECT name FROM users;");

        conn.iterate(&sql, |pairs| {
            for (i, (_, value)) in pairs.iter().enumerate() {
                assert_eq!(*value.as_ref().unwrap(), expects[i]);
            }
            true
        }).unwrap();
    }

    #[test]
    fn iterate_2sets() {
        let conn = prepare();
        let expects = ["Alice", "Bob", "Carol", "Alice", "Bob", "Carol"];
        let sql = conn.ow("SELECT name FROM users; SELECT name FROM users;");

        conn.iterate(&sql, |pairs| {
            for (i, (_, value)) in pairs.iter().enumerate() {
                assert_eq!(*value.as_ref().unwrap(), expects[i]);
            }
            true
        }).unwrap();
    }

    #[test]
    fn iterate_or() {
        let conn = prepare();
        let expects = ["Alice", "Bob"];
        let age = "50";
        let sql = conn.ow("SELECT name FROM users WHERE") +
            &conn.ow("age <") + age + &conn.ow("OR") + age + &conn.ow("< age");

        conn.iterate(&sql, |pairs| {
            for (i, (_, value)) in pairs.iter().enumerate() {
                assert_eq!(*value.as_ref().unwrap(), expects[i]);
            }
            true
        }).unwrap();
    }

    #[test]
    fn rows() {
        let conn = prepare();
        let expects = [("Carol", 50), ("Bob", 69), ("Alice", 42),];
        let sql = conn.ow("SELECT * FROM users;");

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

        conn.rows(&conn.ow("SELECT * FROM users;")).unwrap().iter().enumerate().for_each(|(i, row)| {
            assert_eq!(row.get("name").unwrap(), expects[i].0);
            assert_eq!(row.get("age").unwrap(),  expects[i].1.to_string());
        });
    }

    #[test]
    fn double_quotaion_inside_double_quote() {
        let conn = prepare();
        assert_eq!(
            conn.actual_sql(r#"".ow(""inside str"") -> String""#).unwrap(),
            r#"'".ow(""inside str"") -> String"' "#
        );
        assert_eq!(
            conn.actual_sql(r#"".ow("inside str") -> String""#).unwrap(),
            r#"'".ow("inside str") -> String"' "#
        );
    }

    #[test]
    fn double_quotaion_inside_sigle_quote() {
        let conn = prepare();
        assert_eq!(
            conn.actual_sql(r#""I'm Alice""#).unwrap(),
            r#"'"I''m Alice"' "#
        );
        assert_eq!(
            conn.actual_sql(r#""I''m Alice""#).unwrap(),
            r#"'"I''''m Alice"' "#
        );
    }

    #[test]
    fn single_quotaion_inside_double_quote() {
        let conn = prepare();
        assert_eq!(
            conn.actual_sql(r#"'.ow("inside str") -> String'"#).unwrap(),
            r#"'''.ow("inside str") -> String''' "#
        );
    }

    #[test]
    fn single_quotaion_inside_sigle_quote() {
        let conn = prepare();
        assert_eq!(
            conn.actual_sql("'I''m Alice'").unwrap(),
            r#"'''I''''m Alice''' "#
        );
    }

    #[test]
    fn non_quotaion_inside_sigle_quote() {
        let conn = prepare();
        assert_eq!(
            conn.actual_sql("foo'bar'foo").unwrap(),
            r#"'foo''bar''foo' "#
        );
    }

    #[test]
    fn non_quotaion_inside_double_quote() {
        let conn = prepare();
        assert_eq!(
            conn.actual_sql("foo\"bar\"foo").unwrap(),
            r#"'foo"bar"foo' "#
        );
    }

    #[test]
    fn start_with_quotation_and_end_with_anything_else() {
        let conn = prepare();
        let name = "'Alice'; DROP TABLE users; --";
        let sql = conn.ow("select age from users where name = ") + name + &conn.ow("");
        assert_eq!(
            conn.actual_sql(name).unwrap(),
            r#"'''Alice''; DROP TABLE users; --' "#
        );
        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn whitespace() {
        let conn = prepare();
        let sql = conn.ow("select\n*\rfrom\nusers;");

        conn.iterate(&sql, |_| { true }).unwrap();
    }

    #[test]
    fn sqli_eq_nonquote() {
        let conn = prepare();
        let name = "Alice' or '1'='1";
        let sql = conn.ow("select age from users where name =") + name + &conn.ow(";");
        // "select age from users where name = 'Alice'' or ''1''=''1';"

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn allowlist() {
        let mut conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.add_allowlist(params![42, "foo"]);
        conn.add_allowlist(&[&21, &"bar"]);
        assert_eq!(conn.actual_sql(conn.allowlist(21)), Ok("'21' ".into()));
        assert_eq!(conn.actual_sql(conn.allowlist(42)), Ok("'42' ".into()));
        assert_eq!(conn.actual_sql(conn.allowlist("foo")), Ok("'foo' ".into()));
        assert_eq!(conn.actual_sql(conn.allowlist("bar")), Ok("'bar' ".into()));
    }

    #[test]
    fn int() {
        let conn = prepare();
        let invalid = conn.int("invalid");

        assert_ne!(&invalid, &conn.int(42));
        assert_ne!(&invalid, &conn.int("42"));
        assert_ne!(&invalid, &conn.int("42".to_string()));
        assert_ne!(&invalid, &conn.int(&"42".to_string()));
        assert_eq!(&invalid, &conn.int(std::f64::consts::PI));
        assert_eq!(&invalid, &conn.int('A'));
        assert_eq!(&invalid, &conn.int("str"));
    }

    #[test]
    fn sanitizing() {
        let conn = prepare();
        let name = r#"<script>alert("&1");</script>"#;
        let sql = conn.ow("INSERT INTO users VALUES(") + name + &conn.ow(", 12345);");

        conn.execute(&sql).unwrap();

        conn.rows(conn.ow("SELECT name FROM users WHERE age = 12345;")).unwrap().iter() .all(|row| {
            assert_eq!(
                owsql::html_special_chars(row.get("name").unwrap()),
                "&lt;script&gt;alert(&quot;&amp;1&quot;);&lt;/script&gt;"
            );
            true
        });
    }

    #[test]
    fn error_level() {
        let mut conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.error_level(OwsqlErrorLevel::AlwaysOk).unwrap();
        conn.error_level(OwsqlErrorLevel::Release).unwrap();
        conn.error_level(OwsqlErrorLevel::Develop).unwrap();
        conn.error_level(OwsqlErrorLevel::Debug).unwrap();
    }

    #[test]
    #[allow(non_snake_case)]
    fn error_level_AlwaysOk() {
        let mut conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.error_level(OwsqlErrorLevel::AlwaysOk).unwrap();
        let single_quote = conn.ow("'");
        conn.add_allowlist(params!["Alice"]);
        let name = conn.allowlist("Bob");
        let integer = conn.int("50 or 1=1; --");

        assert_eq!(conn.execute("INVALID SQL"),                     Ok(()));
        assert_eq!(conn.execute("'endless"),                        Ok(()));
        assert_eq!(conn.execute(&single_quote),                     Ok(()));
        assert_eq!(conn.execute(&name),                             Ok(()));
        assert_eq!(conn.execute(&integer),                          Ok(()));
        assert_eq!(conn.iterate("INVALID SQL", |_| unreachable!()), Ok(()));
        assert_eq!(conn.iterate("'endless",    |_| unreachable!()), Ok(()));
        assert_eq!(conn.iterate(&single_quote, |_| unreachable!()), Ok(()));
        assert_eq!(conn.iterate(&name,         |_| unreachable!()), Ok(()));
        assert_eq!(conn.iterate(&integer,      |_| unreachable!()), Ok(()));
        assert_eq!(conn.rows("INVALID SQL"),                        Ok(vec![]));
        assert_eq!(conn.rows("'endless"),                           Ok(vec![]));
        assert_eq!(conn.rows(&single_quote),                        Ok(vec![]));
        assert_eq!(conn.rows(&name),                                Ok(vec![]));
        assert_eq!(conn.rows(&integer),                             Ok(vec![]));
    }

    #[test]
    fn error_level_release() {
        let mut conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.error_level(OwsqlErrorLevel::Release).unwrap();
        let single_quote = conn.ow("'");
        conn.add_allowlist(params!["Alice"]);
        let name = conn.allowlist("Bob");
        let integer = conn.int("50 or 1=1; --");

        assert_eq!(conn.execute("INVALID SQL"),                     err!());
        assert_eq!(conn.execute("'endless"),                        err!());
        assert_eq!(conn.execute(&single_quote),                     err!());
        assert_eq!(conn.execute(&name),                             err!());
        assert_eq!(conn.execute(&integer),                          err!());
        assert_eq!(conn.iterate("INVALID SQL", |_| unreachable!()), err!());
        assert_eq!(conn.iterate("'endless",    |_| unreachable!()), err!());
        assert_eq!(conn.iterate(&single_quote, |_| unreachable!()), err!());
        assert_eq!(conn.iterate(&name,         |_| unreachable!()), err!());
        assert_eq!(conn.iterate(&integer,      |_| unreachable!()), err!());
        assert_eq!(conn.rows("INVALID SQL"),                        err!());
        assert_eq!(conn.rows("'endless"),                           err!());
        assert_eq!(conn.rows(&single_quote),                        err!());
        assert_eq!(conn.rows(&name),                                err!());
        assert_eq!(conn.rows(&integer),                             err!());
    }

    #[test]
    fn error_level_develop() {
        let mut conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.error_level(OwsqlErrorLevel::Develop).unwrap();
        let single_quote = conn.ow("'");
        conn.add_allowlist(params!["Alice"]);
        let name = conn.allowlist("Bob");
        let integer = conn.int("50 or 1=1; --");

        assert_eq!(conn.execute("INVALID SQL"),                     err!("exec error"));
        assert_eq!(conn.execute("'endless"),                        err!("exec error"));
        assert_eq!(conn.execute(&single_quote),                     err!("invalid literal"));
        assert_eq!(conn.execute(&name),                             err!("deny value"));
        assert_eq!(conn.execute(&integer),                          err!("non integer"));
        assert_eq!(conn.iterate("INVALID SQL", |_| unreachable!()), err!("exec error"));
        assert_eq!(conn.iterate("'endless",    |_| unreachable!()), err!("exec error"));
        assert_eq!(conn.iterate(&single_quote, |_| unreachable!()), err!("invalid literal"));
        assert_eq!(conn.iterate(&name,         |_| unreachable!()), err!("deny value"));
        assert_eq!(conn.iterate(&integer,      |_| unreachable!()), err!("non integer"));
        assert_eq!(conn.rows("INVALID SQL"),                        err!("exec error"));
        assert_eq!(conn.rows("'endless"),                           err!("exec error"));
        assert_eq!(conn.rows(&single_quote),                        err!("invalid literal"));
        assert_eq!(conn.rows(&name),                                err!("deny value"));
        assert_eq!(conn.rows(&integer),                             err!("non integer"));
    }

    #[test]
    fn error_level_debug() {
        let mut conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.error_level(OwsqlErrorLevel::Debug).unwrap();
        let single_quote = conn.ow("'");
        conn.add_allowlist(params!["Alice"]);
        let name = conn.allowlist("Bob");
        let integer = conn.int("50 or 1=1; --");

        assert_eq!(conn.execute("INVALID SQL"),
            err!("exec error: MySqlError { ERROR 1064 (42000): You have an error in your SQL syntax; check the manual that corresponds to your MariaDB server version for the right syntax to use near \'\'INVALID SQL\'\' at line 1 }"));
        assert_eq!(conn.execute("'endless"),
            err!("exec error: MySqlError { ERROR 1064 (42000): You have an error in your SQL syntax; check the manual that corresponds to your MariaDB server version for the right syntax to use near \'\'\'\'endless\'\' at line 1 }"));
        assert_eq!(conn.execute(&single_quote),                     err!("invalid literal: '"));
        assert_eq!(conn.execute(&name),                             err!("deny value: Bob"));
        assert_eq!(conn.execute(&integer),                          err!("non integer: 50 or 1=1; --"));
        assert_eq!(conn.iterate("INVALID SQL", |_| unreachable!()),
            err!("exec error: MySqlError { ERROR 1064 (42000): You have an error in your SQL syntax; check the manual that corresponds to your MariaDB server version for the right syntax to use near \'\'INVALID SQL\'\' at line 1 }"));
        assert_eq!(conn.iterate("'endless",    |_| unreachable!()),
            err!("exec error: MySqlError { ERROR 1064 (42000): You have an error in your SQL syntax; check the manual that corresponds to your MariaDB server version for the right syntax to use near \'\'\'\'endless\'\' at line 1 }"));
        assert_eq!(conn.iterate(&single_quote, |_| unreachable!()), err!("invalid literal: '"));
        assert_eq!(conn.iterate(&name,         |_| unreachable!()), err!("deny value: Bob"));
        assert_eq!(conn.iterate(&integer,      |_| unreachable!()), err!("non integer: 50 or 1=1; --"));
        assert_eq!(conn.rows("INVALID SQL"),
            err!("exec error: MySqlError { ERROR 1064 (42000): You have an error in your SQL syntax; check the manual that corresponds to your MariaDB server version for the right syntax to use near \'\'INVALID SQL\'\' at line 1 }"));
        assert_eq!(conn.rows("'endless"),
            err!("exec error: MySqlError { ERROR 1064 (42000): You have an error in your SQL syntax; check the manual that corresponds to your MariaDB server version for the right syntax to use near \'\'\'\'endless\'\' at line 1 }"));
        assert_eq!(conn.rows(&single_quote),                        err!("invalid literal: '"));
        assert_eq!(conn.rows(&name),                                err!("deny value: Bob"));
        assert_eq!(conn.rows(&integer),                             err!("non integer: 50 or 1=1; --"));
    }

    #[test]
    fn integer() {
        let conn = prepare();
        let age = 50;
        let sql = conn.ow("select name from users where age <") + &conn.int(age);

        for row in conn.rows(&sql).unwrap().iter() {
            assert_eq!(row.get("name").unwrap(), "Alice");
        }
    }

    #[test]
    fn ow_into_execute() {
        let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.execute(conn.ow("SELECT") + &conn.int(1)).unwrap();
    }

    #[test]
    fn ow_into_iterate() {
        let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.iterate(conn.ow("SELECT") + &conn.int(1), |_| true ).unwrap();
    }

    #[test]
    fn ow_into_rows() {
        let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        for row in conn.rows(conn.ow("SELECT") + &conn.int(1)).unwrap().iter() {
            assert_eq!(row.get("1").unwrap(), "1");
        }
    }

    #[test]
    fn empty_string() {
        let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        assert_eq!(conn.actual_sql(""), Ok("".to_string()));
    }

    #[test]
    fn executable_comment_syntax() {
        let conn = prepare();
        let sqls = vec![
            (conn.ow("SELECT 1 ") + "/*! +1 */", "SELECT 1  '/*! +1 */' ", "1"),
            (conn.ow("SELECT 1 /*! +1 */"),      "SELECT 1 /*! +1 */ ",    "2"),
        ];

        for (sql, actual_sql, result) in sqls {
            assert_eq!(conn.actual_sql(&sql).unwrap(), actual_sql);
            conn.iterate(&sql, |pairs| {
                for (_, (_, value)) in pairs.iter().enumerate() {
                    assert_eq!(*value.as_ref().unwrap(), result);
                }
                true
            }).unwrap();
        }
    }
}

#[cfg(feature = "mysql")]
#[cfg(not(debug_assertions))]
mod mysql_release_build {
    use owsql::error::*;

    #[test]
    fn error_level_debug_when_release_build() {
        let mut conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
        assert_eq!(
            conn.error_level(OwsqlErrorLevel::Debug),
            Err("OwsqlErrorLevel::Debug cannot be set during release build")
        );
    }

}
