mod macros {
    use concatsql_macro::query;
    use concatsql::prelude::*;

    #[test]
    fn query_test() {
        let name = "foo";
        let sql = query!(r#"SELECT {name}"#);
        assert_eq!(sql.simulate(), "SELECT 'foo'");
        let age = "42 OR 1=1; --";
        let sql = query!(r#"SELECT name FROM users WHERE age = {age}"#);
        assert_eq!(sql.simulate(), "SELECT name FROM users WHERE age = '42 OR 1=1; --'");
        let name = "foo";
        let sql = query!(r#"{name}{name};"#);
        assert_eq!(sql.simulate(), "'foo''foo';");
    }
}
