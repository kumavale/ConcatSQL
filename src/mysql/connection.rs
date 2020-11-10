extern crate mysql_sys as mysql;
use mysql::{Opts, Conn};
use mysql::prelude::*;

use std::cell::RefCell;
use std::pin::Pin;

use crate::Result;
use crate::connection::{Connection, ConcatsqlConn};
use crate::error::{Error, ErrorLevel};
use crate::wrapstring::WrapString;

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
        error_level: ErrorLevel::default(),
    })
}

impl ConcatsqlConn for RefCell<mysql::Conn> {
    fn _execute(&self, s: &WrapString, error_level: &ErrorLevel) -> Result<()> {
        let query = &s.query;
        //let mut conn = self.borrow_mut();
        let conn = &mut *self.borrow_mut();
        match conn.query_drop(&query) {
            Ok(_) => Ok(()),
            Err(e) => Error::new(&error_level, "exec error", &e.to_string()),
        }
    }

    fn _iterate(&self, s: &WrapString, error_level: &ErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>
    {
        let query = &s.query;
        //let mut conn = self.borrow_mut();
        let conn = &mut *self.borrow_mut();
        let mut result = match conn.query_iter(&query) {
            Ok(result) => result,
            Err(e) => return Error::new(&error_level, "exec error", &e.to_string()),
        };

        while let Some(result_set) = result.next_set() {
            let result_set = match result_set {
                Ok(result_set) => result_set,
                Err(e) => return Error::new(&error_level, "exec error", &e.to_string()),
            };
            let mut pairs: Vec<(String, Option<String>)> = Vec::with_capacity(result_set.affected_rows() as usize);

            for row in result_set {
                let row = match row {
                    Ok(row) => row,
                    Err(e) => return Error::new(&error_level, "exec error", &e.to_string()),
                };

                for (i, col) in row.columns().iter().enumerate() {
                    pairs.push((col.name_str().to_string(), row.get(i)));
                }

            }

            let pairs: Vec<(&str, Option<&str>)> = pairs.iter().map(|p| (&*p.0, p.1.as_deref())).collect();
            if !pairs.is_empty() && !callback(&pairs) {
                return Error::new(&error_level, "exec error", "query aborted");
            }
        }

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use crate as concatsql;
    use concatsql::prelude::*;
    use concatsql::error::*;

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
    }
}
