#[cfg(feature = "mysql")]
#[cfg(debug_assertions)]
mod mysql {
    use concatsql::prelude::*;
    use concatsql::{Error, ErrorLevel};

    macro_rules! err {
        () => { Err(Error::AnyError) };
        ($msg:expr) => { Err(Error::Message($msg.to_string())) };
    }

    fn prepare<'a>() -> concatsql::Connection<'a> {
        let conn = concatsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.error_level(ErrorLevel::Debug);
        let stmt = prep!(stmt());
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
        let _conn = concatsql::mysql::open("mysql://localhost:3306/test").unwrap();
    }

    #[test]
    fn execute() {
        let conn = concatsql::mysql::open("mysql://localhost:3306/test").unwrap();
        let stmt = prep!(stmt());
        conn.execute(&stmt).unwrap();
    }

    #[test]
    fn iterate() {
        let conn = prepare();
        let expects = ["Alice", "Bob", "Carol"];
        let sql = prep!("SELECT name FROM users;");

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
        let sql = prep!("SELECT name FROM users; SELECT name FROM users;");

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
        let sql = prep!("SELECT name FROM users WHERE ") +
            &prep!("age < ") + age + &prep!(" OR ") + age + &prep!(" < age");

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
        let expects = [ ("Alice", 42), ("Bob", 69), ("Carol", 50) ];
        let sql = prep!("SELECT * FROM users;");

        let mut cnt = 0;
        let rows = conn.rows(&sql).unwrap();
        for (i, row) in rows.iter().enumerate() {
            cnt += 1;
            assert_eq!(row.get("name").unwrap(), expects[i].0);
            assert_eq!(row.get("age").unwrap(),  expects[i].1.to_string());
        }
        assert!(cnt == expects.len());
    }

    #[test]
    fn rows_foreach() {
        let conn = prepare();
        let expects = [ ("Alice", 42), ("Bob", 69), ("Carol", 50) ];

        let mut cnt = 0;
        conn.rows(&prep!("SELECT * FROM users;")).unwrap().iter().enumerate().for_each(|(i, row)| {
            cnt += 1;
            assert_eq!(row.get("name").unwrap(), expects[i].0);
            assert_eq!(row.get("age").unwrap(),  expects[i].1.to_string());
        });
        assert!(cnt == expects.len());
    }

    #[test]
    fn start_with_quotation_and_end_with_anything_else() {
        let conn = prepare();
        let name = "'Alice'; DROP TABLE users; --";
        let sql = prep!("select age from users where name = ") + name;
        assert_eq!(
            sql.actual_sql(),
            "select age from users where name = '''Alice''; DROP TABLE users; --'"
        );
        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn whitespace() {
        let conn = prepare();
        let sql = prep!("select\n*\rfrom\nusers;");

        conn.iterate(&sql, |_| { true }).unwrap();
    }

    #[test]
    fn sqli_eq_nonquote() {
        let conn = prepare();
        let name = "Alice' or '1'='1";
        let sql = prep!("select age from users where name =") + name + &prep!(";");
        // "select age from users where name = 'Alice'' or ''1''=''1';"

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn sanitizing() {
        let conn = prepare();
        let name = r#"<script>alert("&1");</script>"#;
        let sql = prep!("INSERT INTO users VALUES(") + name + &prep!(", 12345);");

        conn.execute(&sql).unwrap();

        conn.rows(prep!("SELECT name FROM users WHERE age = 12345;")).unwrap().iter() .all(|row| {
            assert_eq!(
                concatsql::html_special_chars(row.get("name").unwrap()),
                "&lt;script&gt;alert(&quot;&amp;1&quot;);&lt;/script&gt;"
            );
            true
        });
    }

    #[test]
    fn error_level() {
        let conn = concatsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.error_level(ErrorLevel::AlwaysOk);
        conn.error_level(ErrorLevel::Release);
        conn.error_level(ErrorLevel::Develop);
        conn.error_level(ErrorLevel::Debug);
    }

    #[test]
    #[allow(non_snake_case)]
    fn error_level_AlwaysOk() {
        let conn = concatsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.error_level(ErrorLevel::AlwaysOk);
        let invalid_sql = "INVALID_SQL";

        assert_eq!(conn.execute(invalid_sql),                      Ok(()));
        assert_eq!(conn.iterate(invalid_sql,  |_| unreachable!()), Ok(()));
        assert_eq!(conn.rows(invalid_sql),                         Ok(vec![]));
    }

    #[test]
    fn error_level_release() {
        let conn = concatsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.error_level(ErrorLevel::Release);
        let invalid_sql = "INVALID_SQL";

        assert_eq!(conn.execute(invalid_sql),                      err!());
        assert_eq!(conn.iterate(invalid_sql,  |_| unreachable!()), err!());
        assert_eq!(conn.rows(invalid_sql),                         err!());
    }

    #[test]
    fn error_level_develop() {
        let conn = concatsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.error_level(ErrorLevel::Develop);
        let invalid_sql = "INVALID_SQL";

        assert_eq!(conn.execute(invalid_sql),                      err!("exec error"));
        assert_eq!(conn.iterate(invalid_sql,  |_| unreachable!()), err!("exec error"));
        assert_eq!(conn.rows(invalid_sql),                         err!("exec error"));
    }

    #[test]
    fn error_level_debug() {
        let conn = concatsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.error_level(ErrorLevel::Debug);
        let invalid_sql = "INVALID_SQL";

        assert_eq!(conn.execute(invalid_sql),
            err!("exec error: MySqlError { ERROR 1064 (42000): You have an error in your SQL syntax; check the manual that corresponds to your MariaDB server version for the right syntax to use near \'INVALID_SQL\' at line 1 }"));
        assert_eq!(conn.iterate(invalid_sql, |_| unreachable!()),
            err!("exec error: MySqlError { ERROR 1064 (42000): You have an error in your SQL syntax; check the manual that corresponds to your MariaDB server version for the right syntax to use near \'INVALID_SQL\' at line 1 }"));
        assert_eq!(conn.rows(invalid_sql),
            err!("exec error: MySqlError { ERROR 1064 (42000): You have an error in your SQL syntax; check the manual that corresponds to your MariaDB server version for the right syntax to use near \'INVALID_SQL\' at line 1 }"));
    }

    #[test]
    fn integer() {
        let conn = prepare();
        let age = 50;
        let sql = prep!("select name from users where age <") + age;

        for row in conn.rows(&sql).unwrap().iter() {
            assert_eq!(row.get("name").unwrap(), "Alice");
        }
    }

    #[test]
    fn prep_into_execute() {
        let conn = concatsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.execute(prep!("SELECT ") + 1).unwrap();
    }

    #[test]
    fn prep_into_iterate() {
        let conn = concatsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.iterate(prep!("SELECT ") + 1, |_| true ).unwrap();
    }

    #[test]
    fn prep_into_rows() {
        let conn = concatsql::mysql::open("mysql://localhost:3306/test").unwrap();
        for row in conn.rows(prep!("SELECT ") + 1).unwrap().iter() {
            assert_eq!(row.get(0).unwrap(),   "1");
            assert_eq!(row.get("?").unwrap(), "1");
        }
    }

    #[test]
    fn executable_comment_syntax() {
        let conn = prepare();
        let sqls = vec![
            //(prep!("SELECT 1 ") + "/*! +1 */", "SELECT 1 '/*! +1 */'", "1"), <- syntax error
            (prep!("SELECT 1 /*! +1 */"),      "SELECT 1 /*! +1 */",   "2"),
        ];

        for (sql, actual_sql, result) in sqls {
            assert_eq!(sql.actual_sql(), actual_sql);
            conn.iterate(&sql, |pairs| {
                for (_, (_, value)) in pairs.iter().enumerate() {
                    assert_eq!(*value.as_ref().unwrap(), result);
                }
                true
            }).unwrap();
        }
    }

    #[test]
    fn like() {
        let conn = prepare();

        let name = "A%";
        let sql = prep!("SELECT * FROM users WHERE name LIKE") + name + prep!(";");

        let mut executed = false;
        conn.rows(&sql).unwrap().iter().all(|row| {
            assert_eq!(row.get("name").unwrap(), "Alice");
            executed = true;
            true
        });
        assert!(executed);

        let name = "A";
        let sql = prep!("SELECT * FROM users WHERE name LIKE ") + ("%".to_owned() + name + "%");
        assert_eq!(sql.actual_sql(), "SELECT * FROM users WHERE name LIKE '%A%'");
        conn.execute(&sql).unwrap();

        let name = "%A%";
        let sql = prep!("SELECT * FROM users WHERE name LIKE ") + ("%".to_owned() + &sanitize_like!(name) + "%");
        assert_eq!(sql.actual_sql(), "SELECT * FROM users WHERE name LIKE '%\\\\%A\\\\%%'");
        conn.execute(&sql).unwrap();

        let name = String::from("%A%");
        let sql = prep!("SELECT * FROM users WHERE name LIKE ") + ("%".to_owned() + &sanitize_like!(name, '$') + "%");
        assert_eq!(sql.actual_sql(), "SELECT * FROM users WHERE name LIKE '%$%A$%%'");
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn multiple_stmt() {
        let conn = prepare();
        let mut cnt = 0;
        for (i, row) in conn.rows("SELECT 1; SELECT 2;").unwrap().iter().enumerate() {
            cnt += 1;
            assert_eq!(row.get_into::<_, i32>(0).unwrap(), [ 1, 2 ][i]);
        };

        let conn = prepare();
        for (i, row) in conn.rows("SELECT age FROM users;").unwrap().iter().enumerate() {
            cnt += 1;
            assert_eq!(row.get_into::<_, i32>(0).unwrap(), [ 42, 69, 50 ][i]);
        };

        assert_eq!(cnt, 5);
    }

    #[test]
    #[ignore]
    fn mass_connection() {
        let capacity = 100;
        let mut conns = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            conns.push(concatsql::mysql::open("mysql://localhost:3306/test").unwrap());
        }
        for i in 1..capacity {
            assert_ne!(conns[0], conns[i]);
        }
    }

    #[test]
    fn blob() {
        let conn = concatsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.execute("CREATE TEMPORARY TABLE b (data blob)").unwrap();
        let data = vec![0x1, 0xA, 0xFF, 0x00, 0x7F];
        let sql = prep!("INSERT INTO b VALUES (") + &data + prep!(")");
        conn.execute(&sql).unwrap();
        for row in conn.rows("SELECT data FROM b").unwrap() {
            assert_eq!(row.get_into::<_, Vec<u8>>(0).unwrap(), data);
        }
    }

    #[test]
    fn question() {
        let conn = prepare();
        let sql = prep!("SELECT name FROM users WHERE name=") + "?";
        for _ in conn.rows(&sql).unwrap() { unreachable!(); }
    }
}

