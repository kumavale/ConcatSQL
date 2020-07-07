#[cfg(feature = "postgres")]
#[cfg(debug_assertions)]
mod postgres {
    #[test]
    fn open() {
        let _conn = owsql::postgres::open("host=localhost user=postgres password=postgres").unwrap();
    }
}
