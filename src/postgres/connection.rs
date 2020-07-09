extern crate postgres_sys as postgres;

use postgres::{Client, NoTls};

use std::collections::HashSet;
use std::cell::RefCell;

use crate::Result;
use crate::OwsqlConn;
use crate::connection::Connection;
use crate::bidimap::BidiMap;
use crate::error::{OwsqlError, OwsqlErrorLevel};
use crate::constants::OW_MINIMUM_LENGTH;
use crate::serial::SerialNumber;
use crate::row::Row;

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
        callback: &mut FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>
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
}

