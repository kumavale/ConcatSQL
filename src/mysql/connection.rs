extern crate mysql_sys as mysql;
use mysql::{Opts, Conn};
use mysql::prelude::*;

use std::collections::HashSet;
use std::cell::RefCell;
use std::fmt;

use crate::Result;
use crate::bidimap::BidiMap;
use crate::error::{OwsqlError, OwsqlErrorLevel};
use crate::constants::OW_MINIMUM_LENGTH;
use crate::overwrite::{IntoInner, overwrite_new};
use crate::serial::SerialNumber;
use crate::parser::{escape_for_allowlist, single_quotaion_escape};
use super::row::MysqlRow;

/// A database connection for MySQL.
pub struct MysqlConnection {
    conn:                   RefCell<mysql::Conn>,
    allowlist:              HashSet<String>,
    serial_number:          RefCell<SerialNumber>,
    ow_len_range:           (usize, usize),
    pub(crate) overwrite:   RefCell<BidiMap<String, String>>,
    pub(crate) error_msg:   RefCell<BidiMap<OwsqlError, String>>,
    pub(crate) error_level: OwsqlErrorLevel,
}

impl PartialEq for MysqlConnection {
    fn eq(&self, other: &Self) -> bool {
        self.conn.as_ptr() == other.conn.as_ptr()
    }
}

impl fmt::Debug for MysqlConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MysqlConnection")
            .field("conn", &self.conn)
            .field("error_level", &self.error_level)
            .finish()
    }
}

impl MysqlConnection {
    /// Open a read-write connection to a new or existing database.
    #[inline]
    pub fn open(url: &str) -> Result<Self> {
        let opts = match Opts::from_url(&url) {
            Ok(opts) => opts,
            Err(e) => return Err(OwsqlError::new(format!("failed to open: {}", e))),
        };

        let conn = match Conn::new(opts) {
            Ok(conn) => conn,
            Err(e) => return Err(OwsqlError::new(format!("failed to open: {}", e))),
        };

        Ok(MysqlConnection {
            conn:          RefCell::new(conn),
            allowlist:     HashSet::new(),
            serial_number: RefCell::new(SerialNumber::default()),
            ow_len_range:  (OW_MINIMUM_LENGTH, OW_MINIMUM_LENGTH),
            overwrite:     RefCell::new(BidiMap::new()),
            error_msg:     RefCell::new(BidiMap::new()),
            //error_level:   OwsqlErrorLevel::default(),
            error_level:   OwsqlErrorLevel::Debug, // for develop
        })
    }

    /// Execute a statement without processing the resulting rows if any.
    #[inline]
    pub fn execute<T: AsRef<str>>(&self, query: T) -> Result<()> {
        let query = match self.convert_to_valid_syntax(query.as_ref()) {
            Ok(query) => query,
            Err(e) => if self.error_level == OwsqlErrorLevel::AlwaysOk {
                return Ok(());
            } else {
                return Err(e);
            },
        };

        match self.conn.borrow_mut().query_drop(&query) {
            Ok(_) => Ok(()),
            Err(e) => self.err("exec error", &e.to_string()),
        }
    }

    #[inline]
    pub fn iterate<T: AsRef<str>, F>(&self, query: T, mut callback: F) -> Result<()>
        where
            F: FnMut(&[(String, Option<String>)]) -> bool,
    {
        let query = match self.convert_to_valid_syntax(query.as_ref()) {
            Ok(query) => query,
            Err(e) => if self.error_level == OwsqlErrorLevel::AlwaysOk {
                return Ok(());
            } else {
                return Err(e);
            },
        };

        let mut conn = self.conn.borrow_mut();
        let mut result = match conn.query_iter(&query) {
            Ok(result) => result,
            Err(e) => return self.err("exec error", &e.to_string()),
        };

        while let Some(result_set) = result.next_set() {
            let mut pairs: Vec<(String, Option<String>)> = Vec::new();
            let result_set = match result_set {
                Ok(result_set) => result_set,
                Err(e) => return self.err("exec error", &e.to_string()),
            };

            let columns = result_set.columns();

            for row in result_set {
                let row = match row {
                    Ok(row) => row,
                    Err(e) => return self.err("exec error", &e.to_string()),
                };

                for (i, col) in row.columns().iter().enumerate() {
                    pairs.push((col.name_str().to_string(), row.get(i)));
                }
            }

            if !callback(&pairs) {
                return self.err("exec error", "query aborted");
            }
        }

        Ok(())
    }

    #[inline]
    pub fn rows<T: AsRef<str>>(&self, query: T) -> Result<Vec<MysqlRow>> {
        let mut rows: Vec<MysqlRow> = Vec::new();

        self.iterate(query, |pairs| {
            let mut row = MysqlRow::new();
            for (column, value) in pairs.iter() {
                row.insert(column.to_string(), value.as_ref().map(|v| v.to_string()));
            }
            rows.push(row);
            true
        })?;

        Ok(rows)
    }

    #[inline]
    pub fn actual_sql<T: AsRef<str>>(&self, query: T) -> Result<String> {
        self.convert_to_valid_syntax(query.as_ref())
    }

    #[inline]
    pub fn ow<T: ?Sized + std::string::ToString>(&self, s: &'static T) -> String {
        let s = s.to_string();
        let result = self.check_valid_literal(&s);
        match result {
            Ok(_) => {
                if !self.overwrite.borrow_mut().contain(&s) {
                    let overwrite = overwrite_new(self.serial_number.borrow_mut().get(), self.ow_len_range);
                    self.overwrite.borrow_mut().insert(s.to_string(), overwrite);
                }
                format!(" {} ", self.overwrite.borrow_mut().get(&s).unwrap())
            },
            Err(e) => {
                if !self.error_msg.borrow_mut().contain(&e) {
                    let overwrite = overwrite_new(self.serial_number.borrow_mut().get(), self.ow_len_range);
                    self.error_msg.borrow_mut().insert(e.clone(), overwrite);
                }
                format!(" {} ", self.error_msg.borrow_mut().get(&e).unwrap())
            },
        }
    }

    #[inline]
    pub(crate) fn err(&self, err_msg: &str, detail_msg: &str) -> Result<(), OwsqlError> {
        match self.error_level {
            OwsqlErrorLevel::AlwaysOk => Ok(()),
            OwsqlErrorLevel::Release  => Err(OwsqlError::AnyError),
            OwsqlErrorLevel::Develop  => Err(OwsqlError::new(&err_msg)),
            OwsqlErrorLevel::Debug    => Err(OwsqlError::new(&format!("{}: {}", err_msg, detail_msg))),
        }
    }
}

