
#[cfg(feature = "sqlite")]
mod sqlite {
    use owsql::params;
    use owsql::error::*;

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
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();
    }

    #[test]
    fn iterate() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let expects = ["Alice", "Bob", "Carol"];
        conn.execute(&conn.ow(stmt())).unwrap();

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
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let expects = ["Alice", "Bob"];
        conn.execute(&conn.ow(stmt())).unwrap();

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
    fn rows() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let expects = [("Alice", 42), ("Bob", 69), ("Carol", 50)];
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
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let expects = [("Alice", 42), ("Bob", 69), ("Carol", 50)];
        conn.execute(&conn.ow(stmt())).unwrap();

        conn.rows(&conn.ow("SELECT * FROM users;")).unwrap().iter().enumerate().for_each(|(i, row)| {
            assert_eq!(row.get("name").unwrap(), expects[i].0);
            assert_eq!(row.get("age").unwrap(),  expects[i].1.to_string());
        });
    }

    #[test]
    fn double_quotaion_inside_double_quote() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();


        let name = r#"".ow(""inside str"") -> String""#;
        let sql = conn.ow("select age from users where name = ") + name;

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn double_quotaion_inside_sigle_quote() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = r#""I'm Alice""#;
        let sql = conn.ow("select age from users where name = ") + name;

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn single_quotaion_inside_double_quote() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = r#"'.ow("inside str") -> String'"#;
        let sql = conn.ow("select age from users where name = ") + name;

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn single_quotaion_inside_sigle_quote() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "'I''m Alice'";
        let sql = conn.ow("select age from users where name = ") + name;

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn non_quotaion_inside_sigle_quote() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "foo'bar'foo";
        let sql = conn.ow("select age from users where name = ") + name;

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn non_quotaion_inside_double_quote() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "foo\"bar\"foo";
        let sql = conn.ow("select age from users where name = ") + name;

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn non_quotaion_inside_double_quote_after_owstring() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let name = "foo\"bar\"foo";
        let sql = conn.ow("select age from users where name = ") + name + &conn.ow("");

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn whitespace() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let sql = conn.ow("select\n*\rfrom\nusers;");

        conn.iterate(&sql, |_| { true }).unwrap();
    }

    #[test]
    fn sqli_eq_nonquote() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
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

    #[test]
    fn error_level() {
        use owsql::error::OwsqlErrorLevel;

        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        conn.error_level(OwsqlErrorLevel::Release);
        conn.error_level(OwsqlErrorLevel::Develop);
        conn.error_level(OwsqlErrorLevel::Debug);
    }

    #[test]
    #[allow(non_snake_case)]
    fn error_level_AlwaysOk() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        conn.error_level(OwsqlErrorLevel::AlwaysOk);
        let single_quote = conn.ow("'");
        conn.add_allowlist(params!["Alice"]);
        let name = conn.allowlist("Bob");
        let integer = conn.int("50 or 1=1; --");

        assert_eq!(conn.execute("INVALID SQL"), Ok(()));
        assert_eq!(conn.execute("'endless"),    Ok(()));
        assert_eq!(conn.execute(&single_quote), Ok(()));
        assert_eq!(conn.execute(&name),         Ok(()));
        assert_eq!(conn.execute(&integer),      Ok(()));
        assert_eq!(conn.iterate("INVALID SQL", |_| unreachable!()), Ok(()));
        assert_eq!(conn.iterate("'endless",    |_| unreachable!()), Ok(()));
        assert_eq!(conn.iterate(&single_quote, |_| unreachable!()), Ok(()));
        assert_eq!(conn.iterate(&name,         |_| unreachable!()), Ok(()));
        assert_eq!(conn.iterate(&integer,      |_| unreachable!()), Ok(()));
        assert_eq!(conn.rows("INVALID SQL"), Ok(vec![]));
        assert_eq!(conn.rows("'endless"),    Ok(vec![]));
        assert_eq!(conn.rows(&single_quote), Ok(vec![]));
        assert_eq!(conn.rows(&name),         Ok(vec![]));
        assert_eq!(conn.rows(&integer),      Ok(vec![]));
    }

    #[test]
    fn error_level_release() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        conn.error_level(OwsqlErrorLevel::Release);
        let single_quote = conn.ow("'");
        conn.add_allowlist(params!["Alice"]);
        let name = conn.allowlist("Bob");
        let integer = conn.int("50 or 1=1; --");

        assert_eq!(conn.execute("INVALID SQL"), Err(OwsqlError::AnyError));
        assert_eq!(conn.execute("'endless"),    Err(OwsqlError::AnyError));
        assert_eq!(conn.execute(&single_quote), Err(OwsqlError::AnyError));
        assert_eq!(conn.execute(&name),         Err(OwsqlError::AnyError));
        assert_eq!(conn.execute(&integer),      Err(OwsqlError::AnyError));
        assert_eq!(conn.iterate("INVALID SQL", |_| unreachable!()), Err(OwsqlError::AnyError));
        assert_eq!(conn.iterate("'endless",    |_| unreachable!()), Err(OwsqlError::AnyError));
        assert_eq!(conn.iterate(&single_quote, |_| unreachable!()), Err(OwsqlError::AnyError));
        assert_eq!(conn.iterate(&name,         |_| unreachable!()), Err(OwsqlError::AnyError));
        assert_eq!(conn.iterate(&integer,      |_| unreachable!()), Err(OwsqlError::AnyError));
        assert_eq!(conn.rows("INVALID SQL"), Err(OwsqlError::AnyError));
        assert_eq!(conn.rows("'endless"),    Err(OwsqlError::AnyError));
        assert_eq!(conn.rows(&single_quote), Err(OwsqlError::AnyError));
        assert_eq!(conn.rows(&name),         Err(OwsqlError::AnyError));
        assert_eq!(conn.rows(&integer),      Err(OwsqlError::AnyError));
    }

    #[test]
    fn error_level_develop() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        conn.error_level(OwsqlErrorLevel::Develop);
        let single_quote = conn.ow("'");
        conn.add_allowlist(params!["Alice"]);
        let name = conn.allowlist("Bob");
        let integer = conn.int("50 or 1=1; --");

        assert_eq!(conn.execute("INVALID SQL"), Err(OwsqlError::Message("exec error".to_string())));
        assert_eq!(conn.execute("'endless"),    Err(OwsqlError::Message("endless".to_string())));
        assert_eq!(conn.execute(&single_quote), Err(OwsqlError::Message("invalid literal".to_string())));
        assert_eq!(conn.execute(&name),         Err(OwsqlError::Message("deny value".to_string())));
        assert_eq!(conn.execute(&integer),      Err(OwsqlError::Message("non integer".to_string())));
        assert_eq!(conn.iterate("INVALID SQL", |_| unreachable!()), Err(OwsqlError::Message("exec error".to_string())));
        assert_eq!(conn.iterate("'endless",    |_| unreachable!()), Err(OwsqlError::Message("endless".to_string())));
        assert_eq!(conn.iterate(&single_quote, |_| unreachable!()), Err(OwsqlError::Message("invalid literal".to_string())));
        assert_eq!(conn.iterate(&name,         |_| unreachable!()), Err(OwsqlError::Message("deny value".to_string())));
        assert_eq!(conn.iterate(&integer,      |_| unreachable!()),  Err(OwsqlError::Message("non integer".to_string())));
        assert_eq!(conn.rows("INVALID SQL"), Err(OwsqlError::Message("exec error".to_string())));
        assert_eq!(conn.rows("'endless"),    Err(OwsqlError::Message("endless".to_string())));
        assert_eq!(conn.rows(&single_quote), Err(OwsqlError::Message("invalid literal".to_string())));
        assert_eq!(conn.rows(&name),         Err(OwsqlError::Message("deny value".to_string())));
        assert_eq!(conn.rows(&integer),      Err(OwsqlError::Message("non integer".to_string())));
    }

    #[test]
    fn error_level_debug() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        conn.error_level(OwsqlErrorLevel::Debug);
        let single_quote = conn.ow("'");
        conn.add_allowlist(params!["Alice"]);
        let name = conn.allowlist("Bob");
        let integer = conn.int("50 or 1=1; --");

        assert_eq!(conn.execute("INVALID SQL"), Err(OwsqlError::Message("exec error: error code: 110".to_string())));
        assert_eq!(conn.execute("'endless"),    Err(OwsqlError::Message("endless: 'endless".to_string())));
        assert_eq!(conn.execute(&single_quote), Err(OwsqlError::Message("invalid literal: '".to_string())));
        assert_eq!(conn.execute(&name),         Err(OwsqlError::Message("deny value: Bob".to_string())));
        assert_eq!(conn.execute(&integer),      Err(OwsqlError::Message("non integer: 50 or 1=1; --".to_string())));
        assert_eq!(conn.iterate("INVALID SQL", |_| unreachable!()), Err(OwsqlError::Message("exec error: error code: 110".to_string())));
        assert_eq!(conn.iterate("'endless",    |_| unreachable!()), Err(OwsqlError::Message("endless: 'endless".to_string())));
        assert_eq!(conn.iterate(&single_quote, |_| unreachable!()), Err(OwsqlError::Message("invalid literal: '".to_string())));
        assert_eq!(conn.iterate(&name,         |_| unreachable!()), Err(OwsqlError::Message("deny value: Bob".to_string())));
        assert_eq!(conn.iterate(&integer,      |_| unreachable!()), Err(OwsqlError::Message("non integer: 50 or 1=1; --".to_string())));
        assert_eq!(conn.rows("INVALID SQL"), Err(OwsqlError::Message("exec error: error code: 110".to_string())));
        assert_eq!(conn.rows("'endless"),    Err(OwsqlError::Message("endless: 'endless".to_string())));
        assert_eq!(conn.rows(&single_quote), Err(OwsqlError::Message("invalid literal: '".to_string())));
        assert_eq!(conn.rows(&name),         Err(OwsqlError::Message("deny value: Bob".to_string())));
        assert_eq!(conn.rows(&integer),      Err(OwsqlError::Message("non integer: 50 or 1=1; --".to_string())));
    }

    #[test]
    fn integer() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();

        let age = 50;
        let sql = conn.ow("select name from users where age <") + &conn.int(age);
        for row in conn.rows(&sql).unwrap().iter() {
            assert_eq!(row.get("name").unwrap(), "Alice");
        }
    }

    #[test]
    fn ow_into_execute() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        conn.execute(conn.ow("SELECT") + &conn.int(1)).unwrap();
    }

    #[test]
    fn ow_into_iterate() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        conn.iterate(conn.ow("SELECT") + &conn.int(1), |_| true ).unwrap();
    }

    #[test]
    fn ow_into_rows() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        for row in conn.rows(conn.ow("SELECT") + &conn.int(1)).unwrap().iter() {
            assert_eq!(row.get("1").unwrap(), "1");
        }
    }

    #[test]
    fn multi_thread() {
        use std::thread;
        use std::sync::{Arc, Mutex};

        let conn = Arc::new(Mutex::new(owsql::sqlite::open(":memory:").unwrap()));
        let stmt = conn.lock().unwrap().ow(stmt());
        conn.lock().unwrap().execute(&stmt).unwrap();

        let mut handles = vec![];

        for i in 0..10 {
            let conn_clone = conn.clone();
            let handle = thread::spawn(move || {
                let conn = &*conn_clone.lock().unwrap();
                let sql = conn.ow("INSERT INTO users VALUES ('Thread', ") + &conn.int(i) + &conn.ow(");");
                conn.execute(&sql).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles { handle.join().unwrap(); }

        let conn = &*conn.lock().unwrap();
        assert_eq!(90, (0..10).map(|mut i| {
            conn.iterate(conn.ow("SELECT age FROM users WHERE age = ") + &conn.int(i), |pairs| {
                pairs.iter().for_each(|(_, v)| { assert_eq!(i.to_string(), v.unwrap()); i*=2; }); true
            }).unwrap(); i
        }).sum::<usize>());
    }

    mod should_panic {
        use owsql::params;
        use super::stmt;

        #[test]
        #[should_panic = "exec error"]
        fn literal() {
            let conn = owsql::sqlite::open(":memory:").unwrap();
            let stmt = conn.ow(stmt());

            conn.execute(&stmt).unwrap();

            let sql = "select * from users;";

            conn.iterate(&sql, |_| { true }).unwrap();
        }

        #[test]
        #[should_panic = "endless"]
        fn endless_string() {
            let conn = owsql::sqlite::open(":memory:").unwrap();
            let stmt = conn.ow(stmt());
            conn.execute(&stmt).unwrap();

            let name = "'endless";
            let sql = conn.ow("select age from users where name =") + name + &conn.ow(";");

            conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
        }

        #[test]
        #[should_panic = "invalid literal"]
        fn sqli_eq_quote() {
            let conn = owsql::sqlite::open(":memory:").unwrap();
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
