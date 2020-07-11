extern crate postgres_sys as postgres;

use postgres::{Client, NoTls};

use std::collections::HashSet;
use std::cell::RefCell;

use crate::Result;
use crate::connection::{Connection, OwsqlConn};
use crate::bidimap::BidiMap;
use crate::error::{OwsqlError, OwsqlErrorLevel};
use crate::constants::OW_MINIMUM_LENGTH;
use crate::serial::SerialNumber;
use crate::parser::escape_string;

/// Open a read-write connection to a new or existing database.
#[inline]
pub fn open(params: &str) -> Result<Connection> {
    let conn = match Client::connect(&params, NoTls) {
        Ok(conn) => conn,
        Err(e) => return Err(OwsqlError::Message(format!("failed to open: {}", e))),
    };

    Ok(Connection {
        conn:          Box::new(RefCell::new(conn)),
        allowlist:     HashSet::new(),
        serial_number: RefCell::new(SerialNumber::default()),
        ow_len_range:  (OW_MINIMUM_LENGTH, OW_MINIMUM_LENGTH),
        overwrite:     RefCell::new(BidiMap::new()),
        error_msg:     RefCell::new(BidiMap::new()),
        error_level:   OwsqlErrorLevel::default(),
    })
}

impl OwsqlConn for RefCell<postgres::Client> {
    fn _execute(&self, query: Result<String>, error_level: &OwsqlErrorLevel) -> Result<()> {
        let query = match query {
            Ok(query) => query,
            Err(e) => if *error_level == OwsqlErrorLevel::AlwaysOk {
                return Ok(());
            } else {
                return Err(e);
            },
        };

        match self.borrow_mut().batch_execute(&query) {
            Ok(_) => Ok(()),
            Err(e) => OwsqlError::new(&error_level, "exec error", &e.to_string()),
        }
    }

    fn _iterate(&self, query: Result<String>, error_level: &OwsqlErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>
    {
        let query = match query {
            Ok(query) => query,
            Err(e) => if *error_level == OwsqlErrorLevel::AlwaysOk {
                return Ok(());
            } else {
                return Err(e);
            },
        };

        let mut conn = self.borrow_mut();
        let statement = match conn.prepare(&query) {
            Ok(stmt) => stmt,
            Err(e) => return OwsqlError::new(&error_level, "exec error", &e.to_string()),
        };

        let rows = match conn.query(&statement, &[]) {
            Ok(result) => result,
            Err(e) => return OwsqlError::new(&error_level, "exec error", &e.to_string()),
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
            return OwsqlError::new(&error_level, "exec error", "query aborted");
        }

        Ok(())
    }

    fn literal_escape(&self, s: &str) -> String {
        escape_string(&s, |c| c == '\'' || c == '\\')
    }
}

#[cfg(test)]
mod tests {
    use crate::error::*;

    #[test]
    fn open() {
        assert!(crate::postgres::open("postgresql://postgres:postgres@localhost").is_ok());
        assert_eq!(
            crate::postgres::open(""),
            Err(OwsqlError::Message("failed to open: invalid configuration: host missing".into()))
        );
        assert_eq!(
            crate::postgres::open("foo\0bar"),
            Err(OwsqlError::Message("failed to open: invalid connection string: unexpected EOF".into()))
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
            conn.execute("\0"),
            Err(OwsqlError::Message("exec error".into())),
        );
        assert_eq!(
            conn.execute("invalid query"),
            Err(OwsqlError::Message("exec error".into())),
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    fn iterate() {
        let conn = crate::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        assert_eq!(
            conn.iterate("\0", |_| { unreachable!(); }),
            Err(OwsqlError::Message("exec error".into())),
        );
        assert_eq!(
            conn.iterate("invalid query", |_| { unreachable!(); }),
            Err(OwsqlError::Message("exec error".into())),
        );
    }
}
