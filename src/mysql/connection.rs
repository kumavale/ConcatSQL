extern crate mysql_sys as mysql;
use mysql::{Opts, Conn};
use mysql::prelude::*;

use std::cell::RefCell;
use std::pin::Pin;

use crate::Result;
use crate::parser::to_hex;
use crate::row::Row;
use crate::connection::{Connection, ConcatsqlConn, ConnKind};
use crate::error::{Error, ErrorLevel};

/// Open a read-write connection to a new or existing database.
pub fn open(url: &str) -> Result<Connection> {
    let opts = match Opts::from_url(&url) {
        Ok(opts) => opts,
        Err(e) => return Err(Error::Message(format!("failed to open: {}", e))),
    };

    let conn = match Conn::new(opts) {
        Ok(conn) => conn,
        Err(e) => return Err(Error::Message(format!("failed to open: {}", e))),
    };

    Ok(Connection {
        conn:        unsafe { Pin::new_unchecked(&*Box::leak(Box::new(RefCell::new(conn)))) },
        error_level: RefCell::new(ErrorLevel::default()),
    })
}

impl ConcatsqlConn for RefCell<mysql::Conn> {
    fn execute_inner(&self, query: &str, error_level: &ErrorLevel) -> Result<()> {
        let mut conn = self.borrow_mut();
        match conn.query_drop(query) {
            Ok(_) => Ok(()),
            Err(e) => Error::new(error_level, "exec error", &e),
        }
    }

    fn iterate_inner(&self, query: &str, error_level: &ErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>
    {
        let mut conn = self.borrow_mut();
        let mut result = match conn.query_iter(query) {
            Ok(result) => result,
            Err(e) => return Error::new(error_level, "exec error", &e),
        };

        while let Some(result_set) = result.next_set() {
            let result_set = match result_set {
                Ok(result_set) => result_set,
                Err(e) => return Error::new(error_level, "exec error", &e),
            };
            let mut pairs: Vec<(String, Option<String>)> = Vec::with_capacity(result_set.affected_rows() as usize);

            for row in result_set {
                let row = match row {
                    Ok(row) => row,
                    Err(e) => return Error::new(error_level, "exec error", &e),
                };

                for (i, col) in row.columns().iter().enumerate() {
                    pairs.push((col.name_str().to_string(), row.get(i)));
                }

            }

            let pairs: Vec<(&str, Option<&str>)> = pairs.iter().map(|p| (&*p.0, p.1.as_deref())).collect();
            if !pairs.is_empty() && !callback(&pairs) {
                return Error::new(error_level, "exec error", "query aborted");
            }
        }

        Ok(())
    }

    fn rows_inner(&self, query: &str, error_level: &ErrorLevel) -> Result<Vec<Row>> {
        let mut conn = self.borrow_mut();
        let mut result = match conn.query_iter(query) {
            Ok(result) => result,
            Err(e) => return Error::new(error_level, "exec error", &e).map(|_|Vec::new()),
        };
        let mut rows: Vec<Row> = Vec::new();

        while let Some(result_set) = result.next_set() {
            let result_set = match result_set {
                Ok(result_set) => result_set,
                Err(e) => return Error::new(error_level, "exec error", &e).map(|_|Vec::new()),
            };

            for result_row in result_set {
                let result_row = match result_row {
                    Ok(row) => row,
                    Err(e) => return Error::new(error_level, "exec error", &e).map(|_|Vec::new()),
                };
                let mut row = Row::new();

                for (i, col) in result_row.columns().iter().enumerate() {
                    let value = match result_row[i] {
                        mysql::Value::Bytes(ref bytes) => {
                            match String::from_utf8(bytes.to_vec()) {
                                Ok(s) => Some(s),
                                Err(_) => Some(to_hex(&bytes)),
                            }
                        }
                        _ => result_row.get(i),
                    };
                    row.insert(col.name_str().to_string(), value);
                }
                rows.push(row);
            }
        }

        Ok(rows)
    }

    fn kind(&self) -> ConnKind {
        ConnKind::MySQL
    }
}


#[cfg(test)]
mod tests {
    use crate as concatsql;
    use concatsql::error::*;
    #[cfg(debug_assertions)]
    use concatsql::prelude::*;

    #[test]
    fn open() {
        assert!(crate::mysql::open("mysql://localhost:3306/test").is_ok());
        assert_eq!(
            crate::mysql::open(""),
            Err(Error::Message("failed to open: URL ParseError { relative URL without a base }".into()))
        );
        assert_eq!(
            crate::mysql::open("foo\0bar"),
            Err(Error::Message("failed to open: URL ParseError { relative URL without a base }".into()))
        );
    }

    #[test]
    fn debug_display() {
        let conn = crate::mysql::open("mysql://localhost:3306/test").unwrap();
        assert_eq!(format!("{:?}", &conn), format!("{:?}", &conn));
    }

    #[test]
    #[cfg(debug_assertions)]
    fn execute() {
        let conn = crate::mysql::open("mysql://localhost:3306/test").unwrap();
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
        let conn = crate::mysql::open("mysql://localhost:3306/test").unwrap();
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
