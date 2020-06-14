#[cfg(feature = "sqlite")]
pub mod sqlite;


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "sqlite")]
    fn sqlite_connect() {
        let _conn = sqlite::open(":memory:").unwrap();
    }
}
