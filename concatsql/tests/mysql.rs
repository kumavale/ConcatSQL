#![allow(clippy::never_loop)]

#[cfg(feature = "mysql")]
#[cfg(debug_assertions)]
mod mysql {
    use concatsql::prelude::*;
    use concatsql::prep;
    use concatsql::{Error, ErrorLevel};

    macro_rules! err {
        () => { Err(Error::AnyError) };
        ($msg:expr) => { Err(Error::Message($msg.to_string())) };
    }

    pub fn prepare() -> concatsql::Connection {
        let conn = concatsql::mysql::open("mysql://localhost:3306/test").unwrap();
        conn.error_level(ErrorLevel::Debug);
        let stmt = prep!(stmt());
        conn.execute(stmt).unwrap();
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
        conn.execute(stmt).unwrap();
    }

    #[test]
    fn iterate() {
        let conn = prepare();
        let expects = ["Alice", "Bob", "Carol"];
        let sql = prep!("SELECT name FROM users;");

        conn.iterate(sql, |pairs| {
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

        conn.iterate(sql, |pairs| {
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

        conn.iterate(sql, |pairs| {
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
            sql.simulate(),
            "select age from users where name = '''Alice''; DROP TABLE users; --'"
        );
        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn whitespace() {
        let conn = prepare();
        let sql = prep!("select\n*\rfrom\nusers;");

        conn.iterate(sql, |_| { true }).unwrap();
    }

    #[test]
    fn sqli_eq_nonquote() {
        let conn = prepare();
        let name = "Alice' or '1'='1";
        let sql = prep!("select age from users where name =") + name + &prep!(";");
        // "select age from users where name = 'Alice'' or ''1''=''1';"

        conn.iterate(sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn sanitizing() {
        let conn = prepare();
        let name = r#"<script>alert("&1");</script>"#;
        let sql = prep!("INSERT INTO users VALUES(") + name + &prep!(", 12345);");

        conn.execute(sql).unwrap();

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
        assert_eq!(conn.rows(invalid_sql),                         Ok(Vec::new()));
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

        for row in conn.rows(&sql).unwrap() {
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
        for row in conn.rows(prep!("SELECT ") + 1).unwrap() {
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
            (prep!("SELECT /*! 42 */"),        "SELECT /*! 42 */",     "42"),
            (prep!("SELECT ") + "/*! 42 */",   "SELECT '/*! 42 */'",   "/*! 42 */"),
        ];

        for (sql, simulate, result) in sqls {
            assert_eq!(sql.simulate(), simulate);
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
        assert_eq!(sql.simulate(), "SELECT * FROM users WHERE name LIKE '%A%'");
        conn.execute(&sql).unwrap();

        let name = "%A%";
        let sql = prep!("SELECT * FROM users WHERE name LIKE ") + ("%".to_owned() + &sanitize_like!(name) + "%");
        assert_eq!(sql.simulate(), "SELECT * FROM users WHERE name LIKE '%\\\\%A\\\\%%'");
        conn.execute(&sql).unwrap();

        let name = String::from("%A%");
        let sql = prep!("SELECT * FROM users WHERE name LIKE ") + ("%".to_owned() + &sanitize_like!(name, '$') + "%");
        assert_eq!(sql.simulate(), "SELECT * FROM users WHERE name LIKE '%$%A$%%'");
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn multiple_stmt() {
        let conn = prepare();
        let mut cnt = 0;
        for (i, row) in conn.rows("SELECT 1; SELECT 2, 3;").unwrap().iter().enumerate() {
                                 /*^^^^^^^^*/// <- only first statement
            cnt += 1;
            assert_eq!(row.get_into::<_, i32>(0).unwrap(), [ 1, 2, 3 ][i]);
        };

        let conn = prepare();
        for (i, row) in conn.rows("SELECT age FROM users;").unwrap().iter().enumerate() {
            cnt += 1;
            assert_eq!(row.get_into::<_, i32>(0).unwrap(), [ 42, 69, 50 ][i]);
        };

        assert_eq!(cnt, 4);
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
        conn.execute(sql).unwrap();
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

    #[test]
    fn map_collect() {
        let conn = prepare();
        let rows = conn.rows("SELECT * FROM users").unwrap();
        let names = rows.iter().map(|row| row.get("name")).collect::<Vec<Option<&str>>>();
        let mut cnt = 0;
        for (i, name) in names.iter().enumerate() {
            cnt += 1;
            assert_eq!(name.unwrap(), ["Alice","Bob","Carol"][i])
        }
        assert_eq!(cnt, 3);
    }

    #[test]
    fn in_array() {
        let conn = prepare();
        let sql = prep!("SELECT * FROM users WHERE name IN (") + vec![] as Vec<&str> + prep!(")");
        conn.rows(&sql).unwrap();
        let sql = prep!("SELECT * FROM users WHERE name IN (") + vec!["Adam"] + prep!(")");
        conn.rows(&sql).unwrap();
        let sql = prep!("SELECT * FROM users WHERE name IN (") + vec!["Adam","Eve"] + prep!(")");
        conn.rows(&sql).unwrap();
    }

    #[test]
    fn uuid() {
        use uuid::Uuid;
        let conn = prepare();
        let sql = prep!("SELECT ") + Uuid::nil();
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(&row[0], "00000000000000000000000000000000");
        }
        let sql = prep!("SELECT ") + Uuid::parse_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap();
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(&row[0], "936DA01F9ABD4D9D80C702AF85C822A8");
        }
    }

    #[test]
    fn sql_injection() {
        let conn = prepare();

        let name = "' OR 1=2; SELECT 1; --";
        let sql = prep!("SELECT age FROM users WHERE name = '") + name + &prep!("';"); // '?' is not placeholder
        assert_eq!(
            conn.rows(&sql),
            Err(Error::Message("exec error: DriverError { Statement takes 0 parameters but 1 was supplied }".to_string()))
        );

        let name = "' OR 1=1; --";
        let sql = prep!("SELECT age FROM users WHERE name = '") + name + &prep!("';"); // '?' is not placeholder
        assert_eq!(
            conn.rows(&sql),
            Err(Error::Message("exec error: DriverError { Statement takes 0 parameters but 1 was supplied }".to_string()))
        );

        let name = "Alice";
        let sql = prep!("SELECT age FROM users WHERE name = '") + name + &prep!("';"); // '?' is not placeholder
        assert_eq!(
            conn.rows(&sql),
            Err(Error::Message("exec error: DriverError { Statement takes 0 parameters but 1 was supplied }".to_string()))
        );

        let name = "'' OR 1=1; --";
        let sql = prep!("SELECT age FROM users WHERE name = ") + name;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        let name = "''; DROP TABLE users; --";
        let sql = prep!("SELECT age FROM users WHERE name = ") + name;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        let sql = prep!("SELECT ") + "0x50 + 0x45";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "0x50 + 0x45");
        }

        let sql = prep!("SELECT ") + "0x414243";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "0x414243");
        }

        let sql = prep!("SELECT ") + "CHAR(0x66)";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "CHAR(0x66)");
        }

        let sql = prep!("SELECT ") + "IF(1=1, 'true', 'false')";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "IF(1=1, 'true', 'false')");
        }

        let sql = prep!("SELECT ") + "na + '-' + me FROM users";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "na + '-' + me FROM users");
        }

        let sql = prep!("SELECT ") + "CONCAT(login, password)";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "CONCAT(login, password)");
        }

        let sql = prep!("SELECT ") + "CONCAT('0x',HEX('c:\\\\boot.ini'))";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "CONCAT('0x',HEX('c:\\\\boot.ini'))");
        }

        let sql = prep!("SELECT ") + "CONCAT(CHAR(75),CHAR(76),CHAR(77))";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "CONCAT(CHAR(75),CHAR(76),CHAR(77))");
        }

        let sql = prep!("SELECT ") + "LOAD_FILE(0x633A5C626F6F742E696E69)";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "LOAD_FILE(0x633A5C626F6F742E696E69)");
        }

        let sql = prep!("SELECT ") + "ASCII('a')";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "ASCII('a')");
        }

        let sql = prep!("SELECT ") + "CHAR(64)";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "CHAR(64)");
        }
    }
}

#[cfg(feature = "mysql")]
mod anti_patterns {
    use concatsql::prep;

    // Although it becomes possible, I do not believe it is less useful
    // because its real advantage is that it still makes it harder to do the wrong thing.
    #[test]
    fn string_to_static_str() {
        let conn = concatsql::mysql::open("mysql://localhost:3306/test").unwrap();
        let sql: &'static str = Box::leak(String::from("SELECT 1").into_boxed_str());  // Leak!
        conn.execute(sql).unwrap();
    }

    #[test]
    fn text_op_integer() {
        let conn = super::mysql::prepare();
        let mut cnt = 0;

        let sql = prep!("SELECT age FROM users WHERE name = ") + i32::MAX;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        let sql = prep!("SELECT age FROM users WHERE name < ") + i32::MAX;
        for _ in conn.rows(&sql).unwrap() {
            cnt += 1;
        }

        let sql = prep!("SELECT age FROM users WHERE name > ") + i32::MAX;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        let sql = prep!("SELECT age FROM users WHERE name = ") + i32::MIN;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        let sql = prep!("SELECT age FROM users WHERE name < ") + i32::MIN;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        let sql = prep!("SELECT age FROM users WHERE name > ") + i32::MIN;
        for _ in conn.rows(&sql).unwrap() {
            cnt += 1;
        }

        let sql = prep!("SELECT age FROM users WHERE name = ") + u32::MAX;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        let sql = prep!("SELECT age FROM users WHERE name < ") + u32::MAX;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        let sql = prep!("SELECT age FROM users WHERE name > ") + u32::MAX;
        for _ in conn.rows(&sql).unwrap() {
            cnt += 1;
        }

        let sql = prep!("SELECT age FROM users WHERE name = ") + u32::MIN;
        for _ in conn.rows(&sql).unwrap() {
            cnt += 1;
        }

        let sql = prep!("SELECT age FROM users WHERE name < ") + u32::MIN;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        let sql = prep!("SELECT age FROM users WHERE name > ") + u32::MIN;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        assert_eq!(cnt, 12);
    }
}

