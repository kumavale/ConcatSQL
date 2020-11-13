extern crate postgres_sys as postgres;

use postgres::{Client, NoTls};

use std::cell::RefCell;
use std::pin::Pin;

use crate::Result;
use crate::row::Row;
use crate::connection::{Connection, ConcatsqlConn, ConnKind};
use crate::error::{Error, ErrorLevel};

/// Open a read-write connection to a new or existing database.
pub fn open(params: &str) -> Result<Connection> {
    let conn = match Client::connect(&params, NoTls) {
        Ok(conn) => conn,
        Err(e) => return Err(Error::Message(format!("failed to open: {}", e))),
    };

    Ok(Connection {
        conn:        unsafe { Pin::new_unchecked(&*Box::leak(Box::new(RefCell::new(conn)))) },
        error_level: RefCell::new(ErrorLevel::default()),
    })
}

impl ConcatsqlConn for RefCell<postgres::Client> {
    fn execute_inner(&self, query: &str, error_level: &ErrorLevel) -> Result<()> {
        match self.borrow_mut().batch_execute(query) {
            Ok(_) => Ok(()),
            Err(e) => Error::new(error_level, "exec error", &e),
        }
    }

    fn iterate_inner(&self, query: &str, error_level: &ErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>
    {
        let mut conn = self.borrow_mut();
        let rows = match conn.query(query, &[]) {
            Ok(result) => result,
            Err(e) => return Error::new(error_level, "exec error", &e),
        };

        let mut pairs = Vec::new();
        for row in rows {
            for (i, col) in row.columns().iter().enumerate() {
                let value = if let Ok(value) = row.try_get::<usize, String>(i) {
                    Some(value)
                } else if let Ok(value) = row.try_get::<usize, i32>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, i64>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, u32>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, f32>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, f64>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, bool>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, i8>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, i16>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, std::net::IpAddr>(i) {
                    Some(value.to_string())
                } else {
                    None
                };

                pairs.push((col.name().to_string(), value));
            }
        }

        let pairs: Vec<(&str, Option<&str>)> = pairs.iter().map(|p| (&*p.0, p.1.as_deref())).collect();
        if !pairs.is_empty() && !callback(&pairs) {
            return Error::new(error_level, "exec error", "query aborted");
        }

        Ok(())
    }

    fn rows_inner(&self, query: &str, error_level: &ErrorLevel) -> Result<Vec<Row>> {
        let mut conn = self.borrow_mut();
        let result = match conn.query(query, &[]) {
            Ok(result) => result,
            Err(e) => return Error::new(error_level, "exec error", &e).map(|_|Vec::new()),
        };

        let mut rows: Vec<Row> = Vec::new();

        for result_row in result {
            let mut row = Row::new();

            for (i, col) in result_row.columns().iter().enumerate() {
                let value = if let Ok(value) = result_row.try_get::<usize, String>(i) {
                    Some(value)
                } else if let Ok(value) = result_row.try_get::<usize, i32>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, i64>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, u32>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, f32>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, f64>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, bool>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, i8>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, i16>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, std::net::IpAddr>(i) {
                    Some(value.to_string())
                } else {
                    None
                };

                row.insert(col.name().to_string(), value);
            }
            rows.push(row);
        }

        Ok(rows)
    }

    fn kind(&self) -> ConnKind {
        ConnKind::PostgreSQL
    }
}


#[cfg(test)]
mod tests {
    use crate as concatsql;
    use concatsql::prelude::*;
    use concatsql::error::*;

    #[test]
    fn open() {
        assert!(crate::postgres::open("postgresql://postgres:postgres@localhost").is_ok());
        assert_eq!(
            crate::postgres::open(""),
            Err(Error::Message("failed to open: invalid configuration: host missing".into()))
        );
        assert_eq!(
            crate::postgres::open("foo\0bar"),
            Err(Error::Message("failed to open: invalid connection string: unexpected EOF".into()))
        );
    }

    #[test]
    fn debug_display() {
        let conn = crate::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        assert_eq!(format!("{:?}", &conn), format!("{:?}", &conn));
    }

    #[test]
    #[cfg(debug_assertions)]
    fn execute() {
        let conn = crate::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        assert_eq!(
            conn.execute(prep!("\0")),
            Err(Error::Message("exec error".into())),
        );
        assert_eq!(
            conn.execute(prep!("invalid query")),
            Err(Error::Message("exec error".into())),
        );
        assert!(conn.execute("SELECT 1").is_ok());
    }

    #[test]
    #[cfg(debug_assertions)]
    fn iterate() {
        let conn = crate::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        assert_eq!(
            conn.iterate(prep!("\0"), |_| { unreachable!(); }),
            Err(Error::Message("exec error".into())),
        );
        assert_eq!(
            conn.iterate(prep!("invalid query"), |_| { unreachable!(); }),
            Err(Error::Message("exec error".into())),
        );
        assert!(conn.iterate("SELECT 1", |_|{true}).is_ok());
    }
}
