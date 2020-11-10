extern crate postgres_sys as postgres;

use postgres::{Client, NoTls};

use std::cell::RefCell;
use std::pin::Pin;

use crate::Result;
use crate::connection::{Connection, ConcatsqlConn};
use crate::error::{Error, ErrorLevel};
use crate::wrapstring::WrapString;

/// Open a read-write connection to a new or existing database.
pub fn open(params: &str) -> Result<Connection> {
    let conn = match Client::connect(&params, NoTls) {
        Ok(conn) => conn,
        Err(e) => return Err(Error::Message(format!("failed to open: {}", e))),
    };

    Ok(Connection {
        conn:        unsafe { Pin::new_unchecked(&*Box::leak(Box::new(RefCell::new(conn)))) },
        error_level: ErrorLevel::default(),
    })
}

impl ConcatsqlConn for RefCell<postgres::Client> {
    fn _execute(&self, s: &WrapString, error_level: &ErrorLevel) -> Result<()> {
        let query = &s.query;
        match self.borrow_mut().batch_execute(&query) {
            Ok(_) => Ok(()),
            Err(e) => Error::new(&error_level, "exec error", &e.to_string()),
        }
    }

    fn _iterate(&self, s: &WrapString, error_level: &ErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>
    {
        let query = &s.query;
        let mut conn = self.borrow_mut();
        let statement = match conn.prepare(&query) {
            Ok(stmt) => stmt,
            Err(e) => return Error::new(&error_level, "exec error", &e.to_string()),
        };

        let rows = match conn.query(&statement, &[]) {
            Ok(result) => result,
            Err(e) => return Error::new(&error_level, "exec error", &e.to_string()),
        };

        let mut pairs = Vec::new();
        for row in rows {
            for col in row.columns() {
                //pairs.push((col.name().to_string(), row.try_get::<&str, String>(col.name()).ok()));
                let value = if let Ok(v) = row.try_get::<&str, String>(col.name()) {
                    Some(v)
                } else if let Ok(v) = row.try_get::<&str, i32>(col.name()) {
                    Some(v.to_string())
                } else {
                    None
                };

                pairs.push((col.name().to_string(), value));
            }
        }

        let pairs: Vec<(&str, Option<&str>)> = pairs.iter().map(|p| (&*p.0, p.1.as_deref())).collect();
        if !pairs.is_empty() && !callback(&pairs) {
            return Error::new(&error_level, "exec error", "query aborted");
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
    }
}
