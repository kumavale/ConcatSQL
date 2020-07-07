extern crate postgres_sys as postgres;

use postgres::{Client, NoTls};

use std::collections::HashSet;
use std::cell::RefCell;
use std::fmt;

use crate::Result;
use crate::OwsqlConn;
use crate::bidimap::BidiMap;
use crate::error::{OwsqlError, OwsqlErrorLevel};
use crate::constants::OW_MINIMUM_LENGTH;
use crate::overwrite::{IntoInner, overwrite_new};
use crate::serial::SerialNumber;
use crate::parser::*;
use crate::row::Row;

/// A database connection for PostgreSQL.
pub struct PostgreSQLConnection {
    conn:                   RefCell<postgres::Client>,
    params:                 String, // tmp
    allowlist:              HashSet<String>,
    serial_number:          RefCell<SerialNumber>,
    ow_len_range:           (usize, usize),
    pub(crate) overwrite:   RefCell<BidiMap<String, String>>,
    pub(crate) error_msg:   RefCell<BidiMap<OwsqlError, String>>,
    pub(crate) error_level: OwsqlErrorLevel,
}

impl PartialEq for PostgreSQLConnection {
    fn eq(&self, other: &Self) -> bool {
        self.conn.as_ptr() == other.conn.as_ptr()
    }
}

impl fmt::Debug for PostgreSQLConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PostgreSQLConnection")
            .field("params", &self.params)
            .field("error_level", &self.error_level)
            .finish()
    }
}

impl PostgreSQLConnection {
    /// Open a read-write connection to a new or existing database.
    #[inline]
    pub fn open(params: &str) -> Result<Self> {
        let conn = match Client::connect(&params, NoTls) {
            Ok(conn) => conn,
            Err(e) => return Err(OwsqlError::new(format!("failed to open: {}", e))),
        };

        Ok(PostgreSQLConnection {
            conn:          RefCell::new(conn),
            params:        params.to_string(),
            allowlist:     HashSet::new(),
            serial_number: RefCell::new(SerialNumber::default()),
            ow_len_range:  (OW_MINIMUM_LENGTH, OW_MINIMUM_LENGTH),
            overwrite:     RefCell::new(BidiMap::new()),
            error_msg:     RefCell::new(BidiMap::new()),
            error_level:   OwsqlErrorLevel::default(),
            //error_level:   dbg!(OwsqlErrorLevel::Debug), // for develop
        })
    }
}
