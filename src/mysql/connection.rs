extern crate mysql_sys as mysql;
use mysql::{Opts, Conn};
use mysql::prelude::*;

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
pub fn open(url: &str) -> Result<Connection> {
    let opts = match Opts::from_url(&url) {
        Ok(opts) => opts,
        Err(e) => return Err(OwsqlError::Message(format!("failed to open: {}", e))),
    };

    let conn = match Conn::new(opts) {
        Ok(conn) => conn,
        Err(e) => return Err(OwsqlError::Message(format!("failed to open: {}", e))),
    };

    Ok(Connection {
        conn:              Box::new(RefCell::new(conn)),
        allowlist:         HashSet::new(),
        serial_number:     RefCell::new(SerialNumber::default()),
        ow_len_range:      (OW_MINIMUM_LENGTH, OW_MINIMUM_LENGTH),
        overwrite:         RefCell::new(BidiMap::new()),
        whitespace_around: RefCell::new(BidiMap::new()),
        error_msg:         RefCell::new(BidiMap::new()),
        error_level:       OwsqlErrorLevel::default(),
    })
}

impl OwsqlConn for RefCell<mysql::Conn> {
    fn _execute(&self, query: Result<String>, error_level: &OwsqlErrorLevel) -> Result<()> {
        let query = match query {
            Ok(query) => query,
            Err(e) => if *error_level == OwsqlErrorLevel::AlwaysOk {
                return Ok(());
            } else {
                return Err(e);
            },
        };

        match self.borrow_mut().query_drop(&query) {
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
        let mut result = match conn.query_iter(&query) {
            Ok(result) => result,
            Err(e) => return OwsqlError::new(&error_level, "exec error", &e.to_string()),
        };

        while let Some(result_set) = result.next_set() {
            let result_set = match result_set {
                Ok(result_set) => result_set,
                Err(e) => return OwsqlError::new(&error_level, "exec error", &e.to_string()),
            };
            let mut pairs: Vec<(String, Option<String>)> = Vec::with_capacity(result_set.affected_rows() as usize);

            for row in result_set {
                let row = match row {
                    Ok(row) => row,
                    Err(e) => return OwsqlError::new(&error_level, "exec error", &e.to_string()),
                };

                for (i, col) in row.columns().iter().enumerate() {
                    pairs.push((col.name_str().to_string(), row.get(i)));
                }

            }

            let pairs: Vec<(&str, Option<&str>)> = pairs.iter().map(|p| (&*p.0, p.1.as_deref())).collect();
            if !pairs.is_empty() && !callback(&pairs) {
                return OwsqlError::new(&error_level, "exec error", "query aborted");
            }
        }

        Ok(())
    }

    fn must_escape(&self) -> Box<dyn Fn(char) -> bool> {
        Box::new(|c| c == '\'' || c == '\\')
    }

    fn literal_escape(&self, s: &str) -> String {
        escape_string(&s, self.must_escape())
    }
}

#[cfg(test)]
mod tests {
    use crate::error::*;

    #[test]
    fn open() {
        assert!(crate::mysql::open("mysql://localhost:3306/test").is_ok());
        assert_eq!(
            crate::mysql::open(""),
            Err(OwsqlError::Message("failed to open: URL ParseError { relative URL without a base }".into()))
        );
        assert_eq!(
            crate::mysql::open("foo\0bar"),
            Err(OwsqlError::Message("failed to open: URL ParseError { relative URL without a base }".into()))
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
        let conn = crate::mysql::open("mysql://localhost:3306/test").unwrap();
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
