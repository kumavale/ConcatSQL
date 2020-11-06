
#[cfg(feature = "sqlite")]
#[cfg(debug_assertions)]
mod sqlite {
    use exowsql::*;

    macro_rules! err {
        () => { Err(exowsql::OwsqlError::AnyError) };
        ($msg:expr) => { Err(exowsql::OwsqlError::Message($msg.to_string())) };
    }

    fn prepare() -> exowsql::Connection {
        let conn = exowsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.prepare(stmt());
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
        let _conn = exowsql::sqlite::open(":memory:").unwrap();
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

        let conn = exowsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.prepare(stmt());
        conn.execute(&stmt).unwrap();
        static_strings! {
            select = "SELECT ";
            cols   = "name ";
            from   = "FROM ";
            table  = "users";
            sql = select!(), cols!(), from!(), table!();
        }
        assert_eq!(conn.prepare(sql).actual_sql(), "SELECT name FROM users");
    }

    #[test]
    fn execute() {
        let conn = exowsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.prepare(stmt());
        conn.execute(&stmt).unwrap();
    }

    #[test]
    fn iterate() {
        let conn = prepare();
        let expects = ["Alice", "Bob", "Carol"];
        let sql = conn.prepare("SELECT name FROM users;");

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
        let sql = conn.prepare("SELECT name FROM users; SELECT name FROM users;");

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
        let sql = conn.prepare("SELECT name FROM users WHERE ") +
            &conn.prepare("age < ") + conn.bind(age) + &conn.prepare(" OR ") + conn.bind(age) + &conn.prepare(" < age");

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
        let sql = conn.prepare("SELECT * FROM users;");

        let rows = conn.rows(&sql).unwrap();
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.get("name").unwrap(), expects[i].0);
            assert_eq!(row.get("age").unwrap(),  expects[i].1.to_string());
        }
    }

    #[test]
    fn rows_foreach() {
        let conn = prepare();
        let expects = [("Alice", 42), ("Bob", 69), ("Carol", 50)];

        conn.rows(&conn.prepare("SELECT * FROM users;")).unwrap().iter().enumerate().for_each(|(i, row)| {
            assert_eq!(row.get("name").unwrap(), expects[i].0);
            assert_eq!(row.get("age").unwrap(),  expects[i].1.to_string());
        });
    }

    #[test]
    fn double_quotaion_inside_double_quote() {
        let conn = prepare();
        assert_eq!(
            conn.bind(r#"".ow(""inside str"") -> String""#).actual_sql(),
            r#"'".ow(""inside str"") -> String"'"#
        );
        assert_eq!(
            conn.bind(r#"".ow("inside str") -> String""#).actual_sql(),
            r#"'".ow("inside str") -> String"'"#
        );
    }

    #[test]
    fn double_quotaion_inside_sigle_quote() {
        let conn = prepare();
        assert_eq!(
            conn.bind(r#""I'm Alice""#).actual_sql(),
            r#"'"I''m Alice"'"#
        );
        assert_eq!(
            conn.bind(r#""I''m Alice""#).actual_sql(),
            r#"'"I''''m Alice"'"#
        );
    }

    #[test]
    fn single_quotaion_inside_double_quote() {
        let conn = prepare();
        assert_eq!(
            conn.bind(r#"'.ow("inside str") -> String'"#).actual_sql(),
            r#"'''.ow("inside str") -> String'''"#
        );
    }

    #[test]
    fn single_quotaion_inside_sigle_quote() {
        let conn = prepare();
        assert_eq!(
            conn.bind("'I''m Alice'").actual_sql(),
            r#"'''I''''m Alice'''"#
        );
    }

    #[test]
    fn non_quotaion_inside_sigle_quote() {
        let conn = prepare();
        assert_eq!(
            conn.bind("foo'bar'foo").actual_sql(),
            r#"'foo''bar''foo'"#
        );
    }

    #[test]
    fn non_quotaion_inside_double_quote() {
        let conn = prepare();
        assert_eq!(
            conn.bind("foo\"bar\"foo").actual_sql(),
            r#"'foo"bar"foo'"#
        );
    }

    #[test]
    fn start_with_quotation_and_end_with_anything_else() {
        let conn = prepare();
        let name = "'Alice'; DROP TABLE users; --";
        let sql = conn.prepare("select age from users where name = ") + conn.bind(name) + &conn.prepare("");
        assert_eq!(
            conn.bind(name).actual_sql(),
            r#"'''Alice''; DROP TABLE users; --'"#
        );
        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
    }

    #[test]
    fn whitespace() {
        let conn = prepare();
        let sql = conn.prepare("select\n*\rfrom\nusers;");

        conn.iterate(&sql, |_| { true }).unwrap();
    }

    #[test]
    fn sqli_eq_nonquote() {
        let conn = prepare();
        let name = "Alice' or '1'='1";
        let sql = conn.prepare("select age from users where name =") + conn.bind(name) + &conn.prepare(";");
        // "select age from users where name = 'Alice'' or ''1''=''1';"

        conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
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
        let sql = conn.prepare("INSERT INTO users VALUES(") + conn.bind(name) + &conn.prepare(", 12345);");

        conn.execute(&sql).unwrap();

        conn.rows(conn.prepare("SELECT name FROM users WHERE age = 12345;")).unwrap().iter() .all(|row| {
            assert_eq!(
                exowsql::html_special_chars(&row.get("name").unwrap()),
                "&lt;script&gt;alert(&quot;&amp;1&quot;);&lt;/script&gt;"
            );
            true
        });
    }

    #[test]
    fn error_level() {
        let mut conn = exowsql::sqlite::open(":memory:").unwrap();
        conn.error_level(OwsqlErrorLevel::AlwaysOk);
        conn.error_level(OwsqlErrorLevel::Release);
        conn.error_level(OwsqlErrorLevel::Develop);
        conn.error_level(OwsqlErrorLevel::Debug);
    }

    #[test]
    #[allow(non_snake_case)]
    fn error_level_AlwaysOk() {
        let mut conn = exowsql::sqlite::open(":memory:").unwrap();
        conn.error_level(OwsqlErrorLevel::AlwaysOk);
        let invalid_sql = conn.bind("INVALID SQL");
        let endless = conn.bind("'endless");

        assert_eq!(conn.execute(&invalid_sql),                      Ok(()));
        assert_eq!(conn.execute(&endless),                          Ok(()));
        assert_eq!(conn.iterate(&invalid_sql,  |_| unreachable!()), Ok(()));
        assert_eq!(conn.iterate(&endless,      |_| unreachable!()), Ok(()));
        assert_eq!(conn.rows(&invalid_sql),                         Ok(vec![]));
        assert_eq!(conn.rows(&endless),                             Ok(vec![]));
    }

    #[test]
    fn error_level_release() {
        let mut conn = exowsql::sqlite::open(":memory:").unwrap();
        conn.error_level(OwsqlErrorLevel::Release);
        let invalid_sql = conn.bind("INVALID SQL");
        let endless = conn.bind("'endless");

        assert_eq!(conn.execute(&invalid_sql),                      err!());
        assert_eq!(conn.execute(&endless),                          err!());
        assert_eq!(conn.iterate(&invalid_sql,  |_| unreachable!()), err!());
        assert_eq!(conn.iterate(&endless,      |_| unreachable!()), err!());
        assert_eq!(conn.rows(&invalid_sql),                         err!());
        assert_eq!(conn.rows(&endless),                             err!());
    }

    #[test]
    fn error_level_develop() {
        let mut conn = exowsql::sqlite::open(":memory:").unwrap();
        conn.error_level(OwsqlErrorLevel::Develop);
        let invalid_sql = conn.bind("INVALID SQL");
        let endless = conn.bind("'endless");

        assert_eq!(conn.execute(&invalid_sql),                      err!("exec error"));
        assert_eq!(conn.execute(&endless),                          err!("exec error"));
        assert_eq!(conn.iterate(&invalid_sql,  |_| unreachable!()), err!("exec error"));
        assert_eq!(conn.iterate(&endless,      |_| unreachable!()), err!("exec error"));
        assert_eq!(conn.rows(&invalid_sql),                         err!("exec error"));
        assert_eq!(conn.rows(&endless),                             err!("exec error"));
    }

    #[test]
    fn error_level_debug() {
        let mut conn = exowsql::sqlite::open(":memory:").unwrap();
        conn.error_level(OwsqlErrorLevel::Debug);
        let invalid_sql = conn.bind("INVALID SQL");
        let endless = conn.bind("'endless");

        assert_eq!(conn.execute(&invalid_sql),
            err!("exec error: near \"\'INVALID SQL\'\": syntax error"));
        assert_eq!(conn.execute(&endless),
            err!("exec error: near \"\'\'\'endless\'\": syntax error"));
        assert_eq!(conn.iterate(&invalid_sql, |_| unreachable!()),
            err!("exec error: near \"\'INVALID SQL\'\": syntax error"));
        assert_eq!(conn.iterate(&endless,     |_| unreachable!()),
            err!("exec error: near \"\'\'\'endless\'\": syntax error"));
        assert_eq!(conn.rows(&invalid_sql),
            err!("exec error: near \"\'INVALID SQL\'\": syntax error"));
        assert_eq!(conn.rows(&endless),
            err!("exec error: near \"\'\'\'endless\'\": syntax error"));
    }

    #[test]
    fn integer() {
        let conn = exowsql::sqlite::open(":memory:").unwrap();
        assert!(conn.int(42).is_ok());
        assert!(conn.int("42").is_ok());
        assert!(conn.int("xxx").is_err());
    }

    #[test]
    fn ow_into_execute() {
        let conn = exowsql::sqlite::open(":memory:").unwrap();
        conn.execute(conn.prepare("SELECT ") + conn.int(1).unwrap()).unwrap();
    }

    #[test]
    fn ow_into_iterate() {
        let conn = exowsql::sqlite::open(":memory:").unwrap();
        conn.iterate(conn.prepare("SELECT ") + conn.int(1).unwrap(), |_| true ).unwrap();
    }

    #[test]
    fn ow_into_rows() {
        let conn = exowsql::sqlite::open(":memory:").unwrap();
        for row in conn.rows(conn.prepare("SELECT ") + conn.int(1).unwrap()).unwrap().iter() {
            assert_eq!(row.get("1").unwrap(), "1");
        }
    }

    #[test]
    fn empty_string() {
        let conn = exowsql::sqlite::open(":memory:").unwrap();
        assert_eq!(conn.prepare("").actual_sql(), "");
    }

    #[test]
    fn multi_thread() {
        use std::thread;
        use std::sync::{Arc, Mutex};

        let conn = Arc::new(Mutex::new(exowsql::sqlite::open(":memory:").unwrap()));
        let stmt = conn.lock().unwrap().prepare(stmt());
        conn.lock().unwrap().execute(&stmt).unwrap();

        let mut handles = vec![];

        for i in 0..10 {
            let conn_clone = conn.clone();
            let handle = thread::spawn(move || {
                let conn = &*conn_clone.lock().unwrap();
                let sql = conn.prepare("INSERT INTO users VALUES ('Thread', ") + conn.int(i).unwrap() + conn.prepare(");");
                conn.execute(&sql).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles { handle.join().unwrap(); }

        let conn = &*conn.lock().unwrap();
        assert_eq!(90, (0..10).map(|mut i| {
            conn.iterate(conn.prepare("SELECT age FROM users WHERE age = ") + &conn.int(i).unwrap(), |pairs| {
                pairs.iter().for_each(|(_, v)| { assert_eq!(i.to_string(), v.unwrap()); i*=2; }); true
            }).unwrap(); i
        }).sum::<usize>());
    }

    #[test]
    fn like() {
        let conn = prepare();

        let name = "A%";
        let sql = conn.prepare("SELECT * FROM users WHERE name LIKE") + conn.bind(name) + conn.prepare(";");

        let mut executed = false;
        conn.rows(&sql).unwrap().iter().all(|row| {
            assert_eq!(row.get("name").unwrap(), "Alice");
            executed = true;
            true
        });
        assert!(executed);

        let name = "A";
        let sql = conn.prepare("SELECT * FROM users WHERE name LIKE ") + conn.bind("%".to_owned() + name + "%");
        assert_eq!(sql.actual_sql(), "SELECT * FROM users WHERE name LIKE '%A%'");
        conn.execute(&sql).unwrap();

        let name = "%A%";
        let sql = conn.prepare("SELECT * FROM users WHERE name LIKE ") + conn.bind("%".to_owned() + &sanitize_like!(name) + "%");
        assert_eq!(sql.actual_sql(), "SELECT * FROM users WHERE name LIKE '%\\%A\\%%'");
        conn.execute(&sql).unwrap();

        let name = String::from("%A%");
        let sql = conn.prepare("SELECT * FROM users WHERE name LIKE ") + conn.bind("%".to_owned() + &sanitize_like!(name, '$') + "%");
        assert_eq!(sql.actual_sql(), "SELECT * FROM users WHERE name LIKE '%$%A$%%'");
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn glob() {
        let conn = prepare();

        let name = "A?['i]*";
        let sql = conn.prepare("SELECT * FROM users WHERE name GLOB") + conn.bind(name) + conn.prepare(";");

        let mut executed = false;
        conn.rows(&sql).unwrap().iter().all(|row| {
            assert_eq!(row.get("name").unwrap(), "Alice");
            executed = true;
            true
        });
        assert!(executed);
    }

    mod should_panic {
        use super::stmt;

        #[test]
        #[should_panic = "exec error"]
        fn literal() {
            let conn = exowsql::sqlite::open(":memory:").unwrap();
            let stmt = conn.prepare(stmt());

            conn.execute(&stmt).unwrap();

            let sql = conn.bind("select * from users;");

            conn.iterate(&sql, |_| { true }).unwrap();
        }

        #[test]
        #[should_panic = "invalid literal"]
        fn sqli_eq_quote() {
            let conn = exowsql::sqlite::open(":memory:").unwrap();
            let stmt = conn.prepare(stmt());
            conn.execute(&stmt).unwrap();

            let name = "OR TRUE; DROP TABLE users; --";
            let sql = conn.prepare("select age from users where name = '") + conn.bind(name) + &conn.prepare("';");

            conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
        }
    }
}

#[cfg(feature = "sqlite")]
#[cfg(not(debug_assertions))]
mod sqlite_release_build {
    use exowsql::error::*;

    #[test]
    fn error_level_debug_when_release_build() {
        let mut conn = exowsql::sqlite::open(":memory:").unwrap();
        assert_eq!(
            conn.error_level(OwsqlErrorLevel::Debug),
            Err("OwsqlErrorLevel::Debug cannot be set during release build")
        );
    }

}
