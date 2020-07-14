
#[cfg(feature = "sqlite")]
#[cfg(debug_assertions)]
mod sqlite {
    use owsql::*;

    macro_rules! err {
        () => { Err(owsql::OwsqlError::AnyError) };
        ($msg:expr) => { Err(owsql::OwsqlError::Message($msg.to_string())) };
    }

    fn prepare() -> owsql::Connection {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
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
        let _conn = owsql::sqlite::open(":memory:").unwrap();
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

        let conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();
        static_strings! {
            select = "SELECT ";
            cols   = "name ";
            from   = "FROM ";
            table  = "users";
            sql = select!(), cols!(), from!(), table!();
        }
        assert_eq!(conn.actual_sql(&conn.ow(sql)), Ok("SELECT name FROM users ".into()));
    }

    #[test]
    fn execute() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt());
        conn.execute(&stmt).unwrap();
    }

    #[test]
    fn iterate() {
        let conn = prepare();
        let expects = ["Alice", "Bob", "Carol"];
        let sql = conn.ow("SELECT name FROM users;");

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
        let sql = conn.ow("SELECT name FROM users; SELECT name FROM users;");

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
        let sql = conn.ow("SELECT name FROM users WHERE") +
            &conn.ow("age <") + age + &conn.ow("OR") + age + &conn.ow("< age");

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
        let expects = [("Alice", 42), ("Bob", 69), ("Carol", 50)];

        conn.rows(&conn.ow("SELECT * FROM users;")).unwrap().iter().enumerate().for_each(|(i, row)| {
            assert_eq!(row.get("name").unwrap(), expects[i].0);
            assert_eq!(row.get("age").unwrap(),  expects[i].1.to_string());
        });
    }

    #[test]
    fn double_quotaion_inside_double_quote() {
        let conn = prepare();
        let name = r#"".ow(""inside str"") -> String""#;  // expect: '".ow(""inside str"") -> String"'
        let sql = conn.ow("select age from users where name = ") + unsafe { &conn.ow_without_html_escape(&name) };
        conn.execute(&sql).unwrap();

        let name = r#"".ow("inside str") -> String""#;  // expect: '".ow("inside str") -> String"'
        let sql = conn.ow("select age from users where name = ") + unsafe { &conn.ow_without_html_escape(&name) };
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn double_quotaion_inside_sigle_quote() {
        let conn = prepare();
        let name = r#""I'm Alice""#; // expect: '"I''m Alice"'
        let sql = conn.ow("select age from users where name = ") + unsafe { &conn.ow_without_html_escape(&name) };
        conn.execute(&sql).unwrap();
        let name = r#""I''m Alice""#; // expect: '"I''''m Alice"'
        let sql = conn.ow("select age from users where name = ") + unsafe { &conn.ow_without_html_escape(&name) };
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn single_quotaion_inside_double_quote() {
        let conn = prepare();
        let name = r#"'.ow("inside str") -> String'"#; // expect: '''.ow("inside str") -> String'''
        let sql = conn.ow("select age from users where name = ") + name;
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn single_quotaion_inside_sigle_quote() {
        let conn = prepare();
        let name = "'I''m Alice'"; // expect: '''I''''m Alice'''
        let sql = conn.ow("select age from users where name = ") + name;
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn non_quotaion_inside_sigle_quote() {
        let conn = prepare();
        let name = "foo'bar'foo"; // expect: 'foo''bar''foo'
        let sql = conn.ow("select age from users where name = ") + name;
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn non_quotaion_inside_double_quote() {
        let conn = prepare();
        let name = "foo\"bar\"foo"; // expect: 'foo\"bar\"foo'
        let sql = conn.ow("select age from users where name = ") + name;
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn non_quotaion_inside_double_quote_after_owstring() {
        let conn = prepare();
        let name = "foo\"bar\"foo"; // expect: 'foo\"bar\"foo'
        let sql = conn.ow("select age from users where name = ") + name + &conn.ow("");
        conn.execute(&sql).unwrap();
    }

    #[test]
    fn start_with_quotation_and_end_with_anything_else() {
        let conn = prepare();
        let name = "'Alice'; DROP TABLE users; --"; // expect: '''Alice''); DROP TABLE users; --'
        let sql = conn.ow("select age from users where name = ") + name + &conn.ow("");

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
        let mut conn = prepare();
        conn.add_allowlist(params![ 30 ]);
        let age = 30;
        let sql = conn.ow("select age from users where age <") + &conn.allowlist(age) + &conn.ow(";");

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
        let sql = conn.ow("INSERT INTO users VALUES(") + name + &conn.ow(", 12345);");

        conn.execute(&sql).unwrap();

        conn.rows(conn.ow("SELECT name FROM users WHERE age = 12345;")).unwrap().iter() .all(|row| {
            assert_eq!(row.get("name").unwrap(), "&lt;script&gt;alert(&quot;&amp;1&quot;);&lt;/script&gt;");
            true
        });
        assert_eq!(
            conn.actual_sql( unsafe { conn.ow_without_html_escape(&name) }),
            Ok(format!("'{}' ", name))
        );
    }

    #[test]
    fn error_level() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        conn.error_level(OwsqlErrorLevel::AlwaysOk).unwrap();
        conn.error_level(OwsqlErrorLevel::Release).unwrap();
        conn.error_level(OwsqlErrorLevel::Develop).unwrap();
        conn.error_level(OwsqlErrorLevel::Debug).unwrap();
    }

    #[test]
    #[allow(non_snake_case)]
    fn error_level_AlwaysOk() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
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
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
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
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
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
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        conn.error_level(OwsqlErrorLevel::Debug).unwrap();
        let single_quote = conn.ow("'");
        conn.add_allowlist(params!["Alice"]);
        let name = conn.allowlist("Bob");
        let integer = conn.int("50 or 1=1; --");

        assert_eq!(conn.execute("INVALID SQL"),
            err!("exec error: near \"\'INVALID SQL\'\": syntax error"));
        assert_eq!(conn.execute("'endless"),
            err!("exec error: near \"\'&#39;endless\'\": syntax error"));
        assert_eq!(conn.execute(&single_quote),                     err!("invalid literal: '"));
        assert_eq!(conn.execute(&name),                             err!("deny value: Bob"));
        assert_eq!(conn.execute(&integer),                          err!("non integer: 50 or 1=1; --"));
        assert_eq!(conn.iterate("INVALID SQL", |_| unreachable!()),
            err!("exec error: near \"\'INVALID SQL\'\": syntax error"));
        assert_eq!(conn.iterate("'endless",    |_| unreachable!()),
            err!("exec error: near \"\'&#39;endless\'\": syntax error"));
        assert_eq!(conn.iterate(&single_quote, |_| unreachable!()), err!("invalid literal: '"));
        assert_eq!(conn.iterate(&name,         |_| unreachable!()), err!("deny value: Bob"));
        assert_eq!(conn.iterate(&integer,      |_| unreachable!()), err!("non integer: 50 or 1=1; --"));
        assert_eq!(conn.rows("INVALID SQL"),
            err!("exec error: near \"\'INVALID SQL\'\": syntax error"));
        assert_eq!(conn.rows("'endless"),
            err!("exec error: near \"\'&#39;endless\'\": syntax error"));
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
    fn empty_string() {
        let conn = owsql::sqlite::open(":memory:").unwrap();
        assert_eq!(conn.actual_sql(""), Ok("".to_string()));
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

    #[test]
    #[allow(clippy::int_plus_one)]
    fn range() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        conn.set_ow_len(1);
        assert_eq!(1+5+32+1+1, conn.ow("0").len());
        conn.set_ow_len(42);
        assert_eq!(1+5+42+1+1, conn.ow("1").len());
        conn.set_ow_len(0..32);
        assert_eq!(1+5+32+1+1, conn.ow("2").len());
        conn.set_ow_len(0..=32);
        assert_eq!(1+5+32+1+1, conn.ow("3").len());
        conn.set_ow_len(64..64);
        assert_eq!(1+5+64+1+1, conn.ow("4").len());
        conn.set_ow_len(64..=64);
        assert_eq!(1+5+64+1+1, conn.ow("5").len());
        conn.set_ow_len(64..32);
        assert!(1+5+33+1+1 <= conn.ow("6").len() && conn.ow("6").len() <= 1+5+64+1+1);
        conn.set_ow_len(64..=32);
        assert!(1+5+32+1+1 <= conn.ow("7").len() && conn.ow("7").len() <= 1+5+64+1+1);
        conn.set_ow_len(100..110);
        assert!(1+5+100+1+1 <= conn.ow("8").len() && conn.ow("8").len() <= 1+5+109+1+1);
        conn.set_ow_len(..64);
        assert!(1+5+32+1+1 <= conn.ow("9").len() && conn.ow("9").len() <= 1+5+63+1+1);
        conn.set_ow_len(..=64);
        assert!(1+5+32+2+1 <= conn.ow("10").len() && conn.ow("10").len() <= 1+5+64+2+1);
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

#[cfg(feature = "sqlite")]
#[cfg(not(debug_assertions))]
mod sqlite_release_build {
    use owsql::error::*;

    #[test]
    fn error_level_debug_when_release_build() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        assert_eq!(
            conn.error_level(OwsqlErrorLevel::Debug),
            Err("OwsqlErrorLevel::Debug cannot be set during release build")
        );
    }

}
