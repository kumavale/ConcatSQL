
#[cfg(feature = "sqlite")]
#[cfg(debug_assertions)]
mod sqlite {
    use concatsql::prelude::*;
    use concatsql::{Error, ErrorLevel};

    macro_rules! err {
        () => { Err(Error::AnyError) };
        ($msg:expr) => { Err(Error::Message($msg.to_string())) };
    }

    pub fn prepare<'a>() -> concatsql::Connection<'a> {
        let conn = concatsql::sqlite::open(":memory:").unwrap();
        conn.error_level(ErrorLevel::Debug);
        let stmt = prep!(stmt());
        conn.execute(&stmt).unwrap();
        conn
    }

    fn stmt() -> &'static str {
        r#"CREATE TABLE users (name TEXT, age INTEGER);
           INSERT INTO users (name, age) VALUES ('Alice', 42);
           INSERT INTO users (name, age) VALUES ('Bob', 69);
           INSERT INTO users (name, age) VALUES ('Carol', 50);"#
    }

    #[test]
    fn open() {
        let _conn = concatsql::sqlite::open(":memory:").unwrap();
    }

    #[test]
    fn static_strings() {
        macro_rules! static_strings {(
            $(
                $var:ident = $($expr:expr),* $(,)? ;
            )*
        ) => (
        $(
            macro_rules! $var {() => (
                concat!($( $expr, )* )
            )}
            #[allow(dead_code, non_upper_case_globals)]
            const $var: &'static str = $var!();
        )*
        )}

        let conn = concatsql::sqlite::open(":memory:").unwrap();
        let stmt = prep!(stmt());
        conn.execute(&stmt).unwrap();
        static_strings! {
            select = "SELECT ";
            cols   = "name ";
            from   = "FROM ";
            table  = "users";
            sql = select!(), cols!(), from!(), table!();
        }
        assert_eq!(prep!(sql).simulate(), "SELECT name FROM users");
    }

    #[test]
    fn execute() {
        let conn = concatsql::sqlite::open(":memory:").unwrap();
        let stmt = prep!(stmt());
        conn.execute(&stmt).unwrap();
    }

    #[test]
    fn iterate() {
        let conn = prepare();
        let expects = ["Alice", "Bob", "Carol"];
        let sql = prep!("SELECT name FROM users;");

        let mut i = 0;
        conn.iterate(&sql, |pairs| {
            for &(_, value) in pairs.iter() {
                assert_eq!(value.unwrap(), expects[i]);
            }
            i += 1;
            true
        }).unwrap();
    }

    #[test]
    fn iterate_2sets() {
        let conn = prepare();
        let expects = ["Alice", "Bob", "Carol", "Alice", "Bob", "Carol"];
        let sql = prep!("SELECT name FROM users; SELECT name FROM users;");

        let mut i = 0;
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
        let conn = prepare();
        let expects = ["Alice", "Bob"];
        let age = "50";
        let sql = prep!("SELECT name FROM users WHERE ") +
            &prep!("age < ") + age + &prep!(" OR ") + age + &prep!(" < age");

        let mut i = 0;
        conn.iterate(&sql, |pairs| {
            for &(_, value) in pairs.iter() {
                assert_eq!(value.unwrap(), expects[i]);
            }
            i += 1;
            true
        }).unwrap();
    }

    #[test]
    fn rows() {
        let conn = prepare();
        let expects = [("Alice", 42), ("Bob", 69), ("Carol", 50)];
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
        let expects = [("Alice", 42), ("Bob", 69), ("Carol", 50)];

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
        let sql = prep!("select age from users where name = ") + name + &prep!("");
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
                concatsql::html_special_chars(&row.get("name").unwrap()),
                "&lt;script&gt;alert(&quot;&amp;1&quot;);&lt;/script&gt;"
            );
            true
        });
    }

    #[test]
    fn error_level() {
        let conn = concatsql::sqlite::open(":memory:").unwrap();
        conn.error_level(ErrorLevel::AlwaysOk);
        conn.error_level(ErrorLevel::Release);
        conn.error_level(ErrorLevel::Develop);
        conn.error_level(ErrorLevel::Debug);
    }

    #[test]
    #[allow(non_snake_case)]
    fn error_level_AlwaysOk() {
        let conn = concatsql::sqlite::open(":memory:").unwrap();
        conn.error_level(ErrorLevel::AlwaysOk);
        let invalid_sql = "INVALID_SQL";

        assert_eq!(conn.execute(invalid_sql),                      Ok(()));
        assert_eq!(conn.iterate(invalid_sql,  |_| unreachable!()), Ok(()));
        assert_eq!(conn.rows(invalid_sql),                         Ok(Vec::new()));
    }

    #[test]
    fn error_level_release() {
        let conn = concatsql::sqlite::open(":memory:").unwrap();
        conn.error_level(ErrorLevel::Release);
        let invalid_sql = "INVALID_SQL";

        assert_eq!(conn.execute(invalid_sql),                      err!());
        assert_eq!(conn.iterate(invalid_sql,  |_| unreachable!()), err!());
        assert_eq!(conn.rows(invalid_sql),                         err!());
    }

    #[test]
    fn error_level_develop() {
        let conn = concatsql::sqlite::open(":memory:").unwrap();
        conn.error_level(ErrorLevel::Develop);
        let invalid_sql = "INVALID_SQL";

        assert_eq!(conn.execute(invalid_sql),                      err!("exec error"));
        assert_eq!(conn.iterate(invalid_sql,  |_| unreachable!()), err!("exec error"));
        assert_eq!(conn.rows(invalid_sql),                         err!("exec error"));
    }

    #[test]
    fn error_level_debug() {
        let conn = concatsql::sqlite::open(":memory:").unwrap();
        conn.error_level(ErrorLevel::Debug);
        let invalid_sql = "INVALID_SQL";

        assert_eq!(conn.execute(invalid_sql),
            err!("exec error: near \"INVALID_SQL\": syntax error"));
        assert_eq!(conn.iterate(invalid_sql, |_| unreachable!()),
            err!("exec error: near \"INVALID_SQL\": syntax error"));
        assert_eq!(conn.rows(invalid_sql),
            err!("exec error: near \"INVALID_SQL\": syntax error"));
    }

    #[test]
    fn prep_into_execute() {
        let conn = concatsql::sqlite::open(":memory:").unwrap();
        conn.execute(prep!("SELECT ") + 1).unwrap();
    }

    #[test]
    fn prep_into_iterate() {
        let conn = concatsql::sqlite::open(":memory:").unwrap();
        conn.iterate(prep!("SELECT ") + 1, |_| true ).unwrap();
    }

    #[test]
    fn prep_into_rows() {
        let conn = concatsql::sqlite::open(":memory:").unwrap();
        let mut executed = false;
        for row in &conn.rows(prep!("SELECT ") + 1).unwrap() {
            executed = true;
            assert_eq!(row.get(0).unwrap(), "1");
        }
        assert!(executed);
    }

    #[test]
    fn multi_thread() {
        use std::thread;
        use std::sync::{Arc, Mutex};

        let conn = Arc::new(Mutex::new(concatsql::sqlite::open(":memory:").unwrap()));
        let stmt = prep!(stmt());
        conn.lock().unwrap().execute(&stmt).unwrap();

        let mut handles = vec![];

        for i in 0..10 {
            let conn_clone = conn.clone();
            let handle = thread::spawn(move || {
                let conn = &*conn_clone.lock().unwrap();
                let sql = prep!("INSERT INTO users VALUES ('Thread', ") + i + prep!(");");
                conn.execute(&sql).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles { handle.join().unwrap(); }

        let conn = &*conn.lock().unwrap();
        assert_eq!(90, (0..10).map(|mut i| {
            conn.iterate(prep!("SELECT age FROM users WHERE age = ") + i, |pairs| {
                pairs.iter().for_each(|(_, v)| { assert_eq!(i.to_string(), v.unwrap()); i*=2; }); true
            }).unwrap(); i
        }).sum::<usize>());
    }

    #[test]
    fn like() {
        let conn = prepare();

        let name = "A%";
        let sql = prep!("SELECT * FROM users WHERE name LIKE ") + name + prep!(";");

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
        if cfg!(feature = "mysql") || cfg!(feature = "postgres") {
            assert_eq!(sql.simulate(), "SELECT * FROM users WHERE name LIKE '%\\\\%A\\\\%%'");
        } else {
            assert_eq!(sql.simulate(), "SELECT * FROM users WHERE name LIKE '%\\%A\\%%'");
        }
        conn.execute(&sql).unwrap();

        let name = String::from("%A%");
        let sql = prep!("SELECT * FROM users WHERE name LIKE ") + ("%".to_owned() + &sanitize_like!(name, '$') + "%");
        assert_eq!(sql.simulate(), "SELECT * FROM users WHERE name LIKE '%$%A$%%'");
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn glob() {
        let conn = prepare();

        let name = "A?['i]*";
        let sql = prep!("SELECT * FROM users WHERE name GLOB ") + name;

        let mut executed = false;
        conn.rows(&sql).unwrap().iter().all(|row| {
            assert_eq!(row.get("name").unwrap(), "Alice");
            executed = true;
            true
        });
        assert!(executed);
    }

    #[test]
    fn multiple_stmt() {
        let conn = prepare();
        let mut cnt = 0;
        for (i, row) in conn.rows("SELECT 1; SELECT 2;").unwrap().iter().enumerate() {
                                 /*^^^^^^^^*/// <- only first statement
            cnt += 1;
            assert_eq!(row.get_into::<_, i32>(0).unwrap(), [ 1, 2 ][i]);
        };
        for (i, row) in conn.rows("SELECT age FROM users;").unwrap().iter().enumerate() {
            cnt += 1;
            assert_eq!(row.get_into::<_, i32>(0).unwrap(), [ 42, 69, 50 ][i]);
        };
        assert_eq!(cnt, 4);
    }

    #[test]
    #[ignore]
    fn mass_connection() {
        let capacity = 65536;
        let mut conns = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            conns.push(concatsql::sqlite::open(":memory:").unwrap());
        }
        for i in 1..capacity {
            assert_ne!(conns[0], conns[i]);
        }
    }

    #[test]
    fn blob() {
        let conn = concatsql::sqlite::open(":memory:").unwrap();
        conn.execute("CREATE TABLE b (data blob)").unwrap();
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

    #[test]
    fn iterator() {
        let conn = prepare();
        let sql = prep!("SELECT name FROM users WHERE name=") + "?";
        for _ in conn.rows(&sql).unwrap() { unreachable!(); }
        for _ in conn.rows(&sql).unwrap().iter() { unreachable!(); }
        for _ in &conn.rows(&sql).unwrap() { unreachable!(); }
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
    fn without_escape() {
        unsafe {
            assert_eq!((prep!() + concatsql::without_escape(&String::from("42")) ).simulate(), "42");
            assert_eq!((prep!() + concatsql::without_escape(&String::from("foo"))).simulate(), "foo");
            assert_eq!((prep!() + concatsql::without_escape(&String::from(""))   ).simulate(), "");
            assert_eq!((prep!() +                            String::from("42")  ).simulate(), "'42'");
            assert_eq!((prep!() +                            String::from("foo") ).simulate(), "'foo'");
            assert_eq!((prep!() +                            String::from("")    ).simulate(), "''");
        }
    }

    #[test]
    fn sql_injection() {
        let conn = prepare();

        let name = "' OR 1=2; SELECT 1; --";
        let sql = prep!("SELECT age FROM users WHERE name = '") + name + &prep!("';"); // '?' is not placeholder
        assert_eq!(
            conn.rows(&sql),
            Err(Error::Message("bind error: column index out of range".to_string()))
        );

        let name = "' OR 1=1; --";
        let sql = prep!("SELECT age FROM users WHERE name = '") + name + &prep!("';"); // '?' is not placeholder
        assert_eq!(
            conn.rows(&sql),
            Err(Error::Message("bind error: column index out of range".to_string()))
        );

        let name = "Alice";
        let sql = prep!("SELECT age FROM users WHERE name = '") + name + &prep!("';"); // '?' is not placeholder
        assert_eq!(
            conn.rows(&sql),
            Err(Error::Message("bind error: column index out of range".to_string()))
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

#[cfg(feature = "sqlite")]
#[cfg(not(debug_assertions))]
mod sqlite_release_build {
    use concatsql::prelude::*;

    #[test]
    fn sqli_enable() {
        let conn = concatsql::sqlite::open(":memory:").unwrap();
        conn.execute("CREATE TABLE users (name TEXT, age INTEGER);").unwrap();

        let name = "OR 1=2; SELECT 1; --";
        let sql = prep!("SELECT age FROM users WHERE name = '") + name + &prep!("';");

        for row in conn.rows(&sql).unwrap() {
            assert_eq!(row.get(0).unwrap(), "1");
        }
    }
}

#[cfg(feature = "sqlite")]
mod anti_patterns {
    use concatsql::prelude::*;

    // Although it becomes possible, I do not believe it is less useful
    // because its real advantage is that it still makes it harder to do the wrong thing.
    #[test]
    fn string_to_static_str() {
        let conn = sqlite::open(":memory:").unwrap();
        let sql: &'static str = Box::leak(String::from("SELECT 1").into_boxed_str());
        conn.execute(sql).unwrap();
    }

    #[test]
    fn text_op_integer() {
        let conn = super::sqlite::prepare();
        let mut cnt = 0;

        let sql = prep!("SELECT age FROM users WHERE name = ") + i32::MAX;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        let sql = prep!("SELECT age FROM users WHERE name < ") + i32::MAX;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        let sql = prep!("SELECT age FROM users WHERE name > ") + i32::MAX;
        for _ in conn.rows(&sql).unwrap() {
            cnt += 1;
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
            unreachable!();
        }

        let sql = prep!("SELECT age FROM users WHERE name < ") + u32::MIN;
        for _ in conn.rows(&sql).unwrap() {
            unreachable!();
        }

        let sql = prep!("SELECT age FROM users WHERE name > ") + u32::MIN;
        for _ in conn.rows(&sql).unwrap() {
            cnt += 1;
        }

        assert_eq!(cnt, 12);
    }
}

