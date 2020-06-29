#[cfg(feature = "mysql")]
mod mysql {

    #[test]
    fn open() {
        let _conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
    }

    //#[test]
    //fn execute() {
    //    let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
    //    let stmt = conn.ow(stmt());
    //    conn.execute(&stmt).unwrap();
    //}
}
