extern crate mysql_sys as mysql;
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
//use super::row::SqliteRow;

/// A database connection for MySQL.
pub struct MysqlConnection {
    conn:                   RefCell<mysql::PooledConn>,
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
        let pool = match mysql::Pool::new(&url) {
            Ok(pool) => pool,
            Err(e) => return Err(OwsqlError::new(format!("failed to open: {}", e))),
        };

        let conn = match pool.get_conn() {
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

