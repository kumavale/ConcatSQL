#![allow(clippy::never_loop)]

#[cfg(feature = "postgres")]
#[cfg(debug_assertions)]
mod postgres {
    use concatsql::prelude::*;
    use concatsql::{Error, ErrorLevel};

    macro_rules! err {
        () => {
            Err(Error::AnyError)
        };
        ($msg:expr) => {
            Err(Error::Message($msg.to_string()))
        };
    }

    const STMT: &str = r#"CREATE TEMPORARY TABLE users (name TEXT, age INTEGER);
           INSERT INTO users (name, age) VALUES ('Alice', 42);
           INSERT INTO users (name, age) VALUES ('Bob', 69);
           INSERT INTO users (name, age) VALUES ('Carol', 50);"#;

    pub fn prepare() -> concatsql::Connection {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.error_level(ErrorLevel::Debug);
        let query = query!("{STMT}");
        conn.execute(query).unwrap();
        conn
    }

    #[test]
    fn open() {
        let _conn =
            concatsql::postgres::open("host=localhost user=postgres password=postgres").unwrap();
        let _conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
    }

    #[test]
    fn execute() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let query = query!("{STMT}");
        conn.execute(query).unwrap();
    }

    #[test]
    fn iterate() {
        let conn = prepare();
        let expects = ["Alice", "Bob", "Carol"];
        let sql = query!("SELECT name FROM users;");

        let mut i = 0;
        conn.iterate(sql, |pairs| {
            for (_, value) in pairs {
                assert_eq!(*value.as_ref().unwrap(), expects[i]);
                i += 1;
            }
            true
        })
        .unwrap();
    }

    #[test]
    #[should_panic = "exec error"]
    fn multiple_stm_should_errort() {
        let conn = prepare();
        let sql = query!("SELECT name FROM users; SELECT name FROM users;");

        conn.iterate(sql, |_| {
            unreachable!();
        })
        .unwrap();
    }

    #[test]
    fn iterate_or() {
        let conn = prepare();
        let expects = ["Alice", "Bob"];
        let age = 50; // "50" error
        let sql = query!("SELECT name FROM users WHERE ")
            + &query!("age < ")
            + age
            + &query!(" OR ")
            + age
            + &query!(" < age");

        let mut i = 0;
        conn.iterate(sql, |pairs| {
            for (_, value) in pairs {
                assert_eq!(*value.as_ref().unwrap(), expects[i]);
                i += 1;
            }
            true
        })
        .unwrap();
    }

    #[test]
    fn rows() {
        let conn = prepare();
        let expects = [("Alice", 42), ("Bob", 69), ("Carol", 50)];
        let sql = query!("SELECT * FROM users;");

        let mut cnt = 0;
        let rows = conn.rows(&sql).unwrap();
        for (i, row) in rows.iter().enumerate() {
            cnt += 1;
            assert_eq!(row.get("name").unwrap(), expects[i].0);
            assert_eq!(row.get("age").unwrap(), expects[i].1.to_string());
        }
        assert!(cnt == expects.len());
    }

    #[test]
    fn rows_foreach() {
        let conn = prepare();
        let expects = [("Alice", 42), ("Bob", 69), ("Carol", 50)];

        let mut cnt = 0;
        conn.rows(query!("SELECT * FROM users;"))
            .unwrap()
            .iter()
            .enumerate()
            .for_each(|(i, row)| {
                cnt += 1;
                assert_eq!(row.get("name").unwrap(), expects[i].0);
                assert_eq!(row.get("age").unwrap(), expects[i].1.to_string());
            });
        assert!(cnt == expects.len());
    }

    #[test]
    fn start_with_quotation_and_end_with_anything_else() {
        let conn = prepare();
        let name = "'Alice'; DROP TABLE users; --";
        let sql = query!("select age from users where name = ") + name + &query!("");
        assert_eq!(
            sql.simulate(),
            "select age from users where name = '''Alice''; DROP TABLE users; --'"
        );
        conn.iterate(&sql, |_| {
            unreachable!();
        })
        .unwrap();
    }

    #[test]
    fn whitespace() {
        let conn = prepare();
        let sql = query!("select\n*\rfrom\nusers;");

        conn.iterate(sql, |_| true).unwrap();
    }

    #[test]
    fn sqli_eq_nonquote() {
        let conn = prepare();
        let name = "Alice' or '1'='1";
        let sql = query!("select age from users where name =") + name + &query!(";");
        // "select age from users where name = 'Alice'' or ''1''=''1';"

        conn.iterate(sql, |_| {
            unreachable!();
        })
        .unwrap();
    }

    #[test]
    fn sanitizing() {
        let conn = prepare();
        let name = r#"<script>alert("&1");</script>"#;
        let sql = query!("INSERT INTO users VALUES(") + name + &query!(", 12345);");

        conn.execute(sql).unwrap();

        conn.rows(query!("SELECT name FROM users WHERE age = 12345;"))
            .unwrap()
            .iter()
            .all(|row| {
                assert_eq!(
                    concatsql::html_special_chars(row.get("name").unwrap()),
                    "&lt;script&gt;alert(&quot;&amp;1&quot;);&lt;/script&gt;"
                );
                true
            });
    }

    #[test]
    fn error_level() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.error_level(ErrorLevel::AlwaysOk);
        conn.error_level(ErrorLevel::Release);
        conn.error_level(ErrorLevel::Develop);
        conn.error_level(ErrorLevel::Debug);
    }

    #[test]
    #[allow(non_snake_case)]
    fn error_level_AlwaysOk() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.error_level(ErrorLevel::AlwaysOk);
        let invalid_sql = "INVALID_SQL";

        assert_eq!(conn.execute(invalid_sql), Ok(()));
        assert_eq!(conn.iterate(invalid_sql, |_| unreachable!()), Ok(()));
        assert_eq!(conn.rows(invalid_sql), Ok(Vec::new()));
    }

    #[test]
    fn error_level_release() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.error_level(ErrorLevel::Release);
        let invalid_sql = "INVALID_SQL";

        assert_eq!(conn.execute(invalid_sql), err!());
        assert_eq!(conn.iterate(invalid_sql, |_| unreachable!()), err!());
        assert_eq!(conn.rows(invalid_sql), err!());
    }

    #[test]
    fn error_level_develop() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.error_level(ErrorLevel::Develop);
        let invalid_sql = "INVALID_SQL";

        assert_eq!(conn.execute(invalid_sql), err!("exec error"));
        assert_eq!(
            conn.iterate(invalid_sql, |_| unreachable!()),
            err!("exec error")
        );
        assert_eq!(conn.rows(invalid_sql), err!("exec error"));
    }

    #[test]
    fn error_level_debug() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.error_level(ErrorLevel::Debug);
        let invalid_sql = "INVALID_SQL";

        assert_eq!(
            conn.execute(invalid_sql),
            err!("exec error: db error: ERROR: \"INVALID_SQL\"またはその近辺で構文エラー")
        );
        assert_eq!(
            conn.iterate(invalid_sql, |_| unreachable!()),
            err!("exec error: db error: ERROR: \"INVALID_SQL\"またはその近辺で構文エラー")
        );
        assert_eq!(
            conn.rows(invalid_sql),
            err!("exec error: db error: ERROR: \"INVALID_SQL\"またはその近辺で構文エラー")
        );
    }

    #[test]
    fn integer() {
        let conn = prepare();
        let age = 50;
        let sql = query!("select name from users where age < ") + age;

        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get("name").unwrap(), "Alice");
        }
    }

    #[test]
    fn prep_into_execute() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.execute(query!("SELECT ") + 1 + query!("::INTEGER"))
            .unwrap();
    }

    #[test]
    fn prep_into_iterate() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.iterate(query!("SELECT ") + 1 + query!("::INTEGER"), |_| true)
            .unwrap();
    }

    #[test]
    fn prep_into_rows() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        for row in conn
            .rows(query!("SELECT ") + 1 + query!("::INTEGER"))
            .unwrap()
        {
            assert_eq!(row.column_name(0).unwrap(), "int4");
            assert_eq!(row.get("int4").unwrap(), "1");
            assert_eq!(row.get(0).unwrap(), "1");
        }
        for row in conn
            .rows(query!("SELECT ") + "1" + query!("::TEXT"))
            .unwrap()
        {
            assert_eq!(row.column_name(0).unwrap(), "text");
            assert_eq!(row.get("text").unwrap(), "1");
            assert_eq!(row.get(0).unwrap(), "1");
        }
        for row in conn.rows(query!("SELECT 1")).unwrap() {
            assert_eq!(row.column_name(0).unwrap(), "?column?");
            assert_eq!(row.get("?column?").unwrap(), "1");
            assert_eq!(row.get(0).unwrap(), "1");
        }
    }

    #[test]
    fn like() {
        let conn = prepare();

        let name = "A%";
        let sql = query!("SELECT * FROM users WHERE name LIKE ") + name;

        let mut executed = false;
        conn.rows(&sql).unwrap().iter().all(|row| {
            assert_eq!(row.get("name").unwrap(), "Alice");
            executed = true;
            true
        });
        assert!(executed);

        let name = "A";
        let sql = query!("SELECT * FROM users WHERE name LIKE ") + ("%".to_owned() + name + "%");
        assert_eq!(sql.simulate(), "SELECT * FROM users WHERE name LIKE '%A%'");
        conn.execute(&sql).unwrap();

        let name = "%A%";
        let sql = query!("SELECT * FROM users WHERE name LIKE ")
            + ("%".to_owned() + &sanitize_like!(name) + "%");
        assert_eq!(
            sql.simulate(),
            "SELECT * FROM users WHERE name LIKE '%\\\\%A\\\\%%'"
        );
        conn.execute(&sql).unwrap();

        let name = String::from("%A%");
        let sql = query!("SELECT * FROM users WHERE name LIKE ")
            + ("%".to_owned() + &sanitize_like!(name, '$') + "%");
        assert_eq!(
            sql.simulate(),
            "SELECT * FROM users WHERE name LIKE '%$%A$%%'"
        );
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn multiple_stmt() {
        let conn = prepare();
        let mut cnt = 0;
        for (i, row) in conn
            .rows("SELECT 1 UNION SELECT 2;")
            .unwrap()
            .iter()
            .enumerate()
        {
            cnt += 1;
            assert_eq!(row.get_into::<_, i32>(0).unwrap(), [1, 2][i]);
        }

        for (i, row) in conn
            .rows("SELECT age FROM users;")
            .unwrap()
            .iter()
            .enumerate()
        {
            cnt += 1;
            assert_eq!(row.get_into::<_, i32>(0).unwrap(), [42, 69, 50][i]);
        }

        assert_eq!(cnt, 5);
    }

    #[test]
    #[ignore]
    fn mass_connection() {
        let capacity = 64;
        let mut conns = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            conns.push(
                concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap(),
            );
        }
        for i in 1..capacity {
            assert_ne!(conns[0], conns[i]);
        }
    }

    #[test]
    fn blob() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        conn.execute("CREATE TEMPORARY TABLE b (data bytea)")
            .unwrap();
        let data = vec![0x1, 0xA, 0xFF, 0x00, 0x7F];
        let sql = query!("INSERT INTO b VALUES (") + &data + query!(")");
        conn.execute(sql).unwrap();
        for row in conn.rows("SELECT data FROM b").unwrap() {
            assert_eq!(row.get_into::<_, Vec<u8>>(0).unwrap(), data);
        }
    }

    #[test]
    fn question() {
        let conn = prepare();
        let sql = query!("SELECT name FROM users WHERE name=") + "?";
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }
    }

    #[test]
    fn map_collect() {
        let conn = prepare();
        let rows = conn.rows("SELECT * FROM users").unwrap();
        let names = rows
            .iter()
            .map(|row| row.get("name"))
            .collect::<Vec<Option<&str>>>();
        let mut cnt = 0;
        for (i, name) in names.iter().enumerate() {
            cnt += 1;
            assert_eq!(name.unwrap(), ["Alice", "Bob", "Carol"][i])
        }
        assert_eq!(cnt, 3);
    }

    #[test]
    fn in_array() {
        let conn = prepare();
        let sql = query!("SELECT * FROM users WHERE name IN (") + vec![] as Vec<&str> + query!(")");
        conn.rows(&sql).unwrap();
        let sql = query!("SELECT * FROM users WHERE name IN (") + vec!["Adam"] + query!(")");
        conn.rows(&sql).unwrap();
        let sql = query!("SELECT * FROM users WHERE name IN (") + vec!["Adam", "Eve"] + query!(")");
        conn.rows(&sql).unwrap();
    }

    #[test]
    fn uuid() {
        use uuid::Uuid;
        let conn = prepare();
        let sql = query!("SELECT ") + Uuid::nil();
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(&row[0], "00000000000000000000000000000000");
        }
        let sql =
            query!("SELECT ") + Uuid::parse_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap();
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(&row[0], "936DA01F9ABD4D9D80C702AF85C822A8");
        }
    }

    #[test]
    fn sql_injection() {
        let conn = prepare();

        let name = "'' OR 1=1; --";
        let sql = query!("SELECT age FROM users WHERE name = ") + name;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        let name = "''; DROP TABLE users; --";
        let sql = query!("SELECT age FROM users WHERE name = ") + name;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        let sql = query!("SELECT ") + "0x50 + 0x45";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "0x50 + 0x45");
        }

        let sql = query!("SELECT ") + "0x414243";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "0x414243");
        }

        let sql = query!("SELECT ") + "CHAR(0x66)";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "CHAR(0x66)");
        }

        let sql = query!("SELECT ") + "IF(1=1, 'true', 'false')";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "IF(1=1, 'true', 'false')");
        }

        let sql = query!("SELECT ") + "na + '-' + me FROM users";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "na + '-' + me FROM users");
        }

        let sql = query!("SELECT ") + "ASCII('a')";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "ASCII('a')");
        }

        let sql = query!("SELECT ") + "CHAR(64)";
        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "CHAR(64)");
        }
    }
}

#[cfg(feature = "postgres")]
mod anti_patterns {
    use concatsql::prelude::*;

    // Although it becomes possible, I do not believe it is less useful
    // because its real advantage is that it still makes it harder to do the wrong thing.
    #[test]
    fn string_to_static_str() {
        let conn = concatsql::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        let sql: &'static str = Box::leak(String::from("SELECT 1").into_boxed_str()); // Leak!
        conn.execute(sql).unwrap();
    }

    #[test]
    fn text_op_integer() {
        let conn = super::postgres::prepare();

        let sql = query!("SELECT age FROM users WHERE name = ") + i32::MAX;
        assert!(conn.rows(&sql).is_err());

        let sql = query!("SELECT age FROM users WHERE name < ") + i32::MAX;
        assert!(conn.rows(&sql).is_err());

        let sql = query!("SELECT age FROM users WHERE name > ") + i32::MAX;
        assert!(conn.rows(&sql).is_err());

        let sql = query!("SELECT age FROM users WHERE name = ") + i32::MIN;
        assert!(conn.rows(&sql).is_err());

        let sql = query!("SELECT age FROM users WHERE name < ") + i32::MIN;
        assert!(conn.rows(&sql).is_err());

        let sql = query!("SELECT age FROM users WHERE name > ") + i32::MIN;
        assert!(conn.rows(&sql).is_err());

        let sql = query!("SELECT age FROM users WHERE name = ") + u32::MAX;
        assert!(conn.rows(&sql).is_err());

        let sql = query!("SELECT age FROM users WHERE name < ") + u32::MAX;
        assert!(conn.rows(&sql).is_err());

        let sql = query!("SELECT age FROM users WHERE name > ") + u32::MAX;
        assert!(conn.rows(&sql).is_err());

        let sql = query!("SELECT age FROM users WHERE name = ") + u32::MIN;
        assert!(conn.rows(&sql).is_err());

        let sql = query!("SELECT age FROM users WHERE name < ") + u32::MIN;
        assert!(conn.rows(&sql).is_err());

        let sql = query!("SELECT age FROM users WHERE name > ") + u32::MIN;
        assert!(conn.rows(&sql).is_err());
    }

    #[test]
    #[should_panic = "expected 0 parameters but got 1"]
    fn invalid_placeholders() {
        let conn = super::postgres::prepare();

        let name = "' OR 1=2; SELECT 1; --";
        let sql = query!("SELECT age FROM users WHERE name = '") + name + &query!("';"); // '?' is not placeholder
        conn.execute(sql).ok();

        let name = "' OR 1=1; --";
        let sql = query!("SELECT age FROM users WHERE name = '") + name + &query!("';"); // '?' is not placeholder
        conn.execute(sql).ok();

        let name = "Alice";
        let sql = query!("SELECT age FROM users WHERE name = '") + name + &query!("';"); // '?' is not placeholder
        conn.execute(sql).ok();
    }
}
