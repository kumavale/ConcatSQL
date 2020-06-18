
#[cfg(feature = "sqlite")]
mod sqlite {
    extern crate owsql;
    use owsql::Result;

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
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt()).unwrap();
        conn.execute(&stmt).unwrap();
    }

    #[test]
    fn iterate() {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt()).unwrap();
        let expects = ["Alice", "Bob", "Carol"];

        conn.execute(&stmt).unwrap();

        let mut i = 0;
        let sql = conn.ow("SELECT name FROM users;").unwrap();

        conn.iterate(&sql, |pairs| {
            for &(_, value) in pairs.iter() {
                assert_eq!(value.unwrap(), expects[i]);
            }
            i += 1;
            true
        }).unwrap();
    }

    #[test]
    fn iterate_or() -> Result<()> {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt())?;
        let expects = ["Alice", "Bob"];

        conn.execute(&stmt).unwrap();

        let mut i = 0;
        let age = "50";
        let sql = conn.ow("SELECT name FROM users WHERE")? +
            &conn.ow("age <")? + age + &conn.ow("OR")? + age + &conn.ow("< age")?;

        conn.iterate(&sql, |pairs| {
            for &(_, value) in pairs.iter() {
                assert_eq!(value.unwrap(), expects[i]);
            }
            i += 1;
            true
        })
    }

    #[test]
    fn double_quotaion_inside_double_quote() -> Result<()> {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt())?;
        conn.execute(&stmt).unwrap();


        let name = r#"".ow(""inside str"") -> String""#;
        let sql = conn.ow("select age from users where name = ")? + name;

        conn.iterate(&sql, |_| { unreachable!(); })
    }

    #[test]
    fn double_quotaion_inside_sigle_quote() -> Result<()> {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt())?;
        conn.execute(&stmt).unwrap();

        let name = r#""I'm Alice""#;
        let sql = conn.ow("select age from users where name = ")? + name;

        conn.iterate(&sql, |_| { unreachable!(); })
    }

    #[test]
    fn single_quotaion_inside_double_quote() -> Result<()>{
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt())?;
        conn.execute(&stmt).unwrap();

        let name = r#"'.ow("inside str") -> String'"#;
        let sql = conn.ow("select age from users where name = ")? + name;

        conn.iterate(&sql, |_| { unreachable!(); })
    }

    #[test]
    fn single_quotaion_inside_sigle_quote() -> Result<()> {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt())?;
        conn.execute(&stmt).unwrap();

        let name = "'I''m Alice'";
        let sql = conn.ow("select age from users where name = ")? + name;

        conn.iterate(&sql, |_| { unreachable!(); })
    }

    #[test]
    fn whitespace() -> Result<()>{
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt())?;
        conn.execute(&stmt).unwrap();

        let sql = conn.ow("select\n*\rfrom\nusers;")?;

        conn.iterate(&sql, |_| { true })
    }

    #[test]
    fn sqli_eq_nonquote() -> Result<()> {
        let mut conn = owsql::sqlite::open(":memory:").unwrap();
        let stmt = conn.ow(stmt())?;
        conn.execute(&stmt).unwrap();

        let name = "Alice' or '1'='1";
        let sql = conn.ow("select age from users where name =")? + name + &conn.ow(";")?;
        // "select age from users where name = 'Alice'' or ''1''=''1';"

        conn.iterate(&sql, |_| { unreachable!(); })
    }

    mod should_panic {
        use super::stmt;

        #[test]
        #[should_panic = "exec error"]
        fn literal() {
            let mut conn = owsql::sqlite::open(":memory:").unwrap();
            let stmt = conn.ow(stmt()).unwrap();

            conn.execute(&stmt).unwrap();

            let sql = "select * from users;";

            conn.iterate(&sql, |_| { true }).unwrap();
        }

        #[test]
        #[should_panic = "endless"]
        fn endless_string() {
            let mut conn = owsql::sqlite::open(":memory:").unwrap();
            let stmt = conn.ow(stmt()).unwrap();
            conn.execute(&stmt).unwrap();

            let name = "'endless";
            let sql = conn.ow("select age from users where name =").unwrap() + name + &conn.ow(";").unwrap();

            conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
        }

        #[test]
        #[should_panic = "invalid literal"]
        fn sqli_eq_quote() {
            let mut conn = owsql::sqlite::open(":memory:").unwrap();
            let stmt = conn.ow(stmt()).unwrap();
            conn.execute(&stmt).unwrap();

            let name = "OR TRUE; DROP TABLE users; --";
            let sql = conn.ow("select age from users where name = '").unwrap() + name + &conn.ow("';").unwrap();

            conn.iterate(&sql, |_| { unreachable!(); }).unwrap();
        }
    }
}
