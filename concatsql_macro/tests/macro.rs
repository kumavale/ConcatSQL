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
        let sql = query!(r#"{{name}}"#);
        assert_eq!(sql.simulate(), "{name}");
    }

    #[test]
    fn query_value_test() {
        let i32_: i32 = 1;
        let i64_: i64 = 2;
        let f32_: f32 = 3.;
        let f64_: f64 = 4.;
        let text: String = "5".to_string();

        let sql = query!(r#"{i32_}{i64_}{f32_}{f64_}{text}"#);
        assert_eq!(sql.simulate(), "1234'5'");
    }

    //#[test]
    //fn query_compile_error_not_found_test() {
    //    _ = query!(r#"{var}"#);
    //}
}
