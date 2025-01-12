extern crate postgres_sys as postgres;

use postgres::{Client, NoTls};
use uuid::Uuid;

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::time::SystemTime;

use crate::connection::{ConcatsqlConn, ConnKind, Connection};
use crate::error::{Error, ErrorLevel};
use crate::row::Row;
use crate::value::{SystemTimeToString, Value};
use crate::Result;

/// Open a read-write connection to a new or existing database.
pub fn open(params: &str) -> Result<Connection> {
    let conn = match Client::connect(params, NoTls) {
        Ok(conn) => conn,
        Err(e) => return Err(Error::Message(format!("failed to open: {}", e))),
    };

    Ok(Connection {
        conn: Box::new(RefCell::new(conn)),
        error_level: Cell::new(ErrorLevel::default()),
    })
}

macro_rules! to_sql {
    ($value:expr) => {
        match $value {
            Value::Null => &"NULL" as &(dyn postgres::types::ToSql + Sync),
            Value::I32(value) => value,
            Value::I64(value) => value,
            Value::F32(value) => value,
            Value::F64(value) => value,
            Value::Text(value) => value,
            Value::Bytes(value) => value,
            Value::IpAddr(value) => value,
            Value::Time(value) => value,
        }
    };
}

impl ConcatsqlConn for RefCell<postgres::Client> {
    fn execute_inner<'a>(
        &self,
        query: Cow<'a, str>,
        params: &[Value<'a>],
        error_level: &ErrorLevel,
    ) -> Result<()> {
        if params.is_empty() {
            match self.borrow_mut().batch_execute(&query) {
                Ok(_) => Ok(()),
                Err(e) => Error::new(error_level, "exec error", &e),
            }
        } else {
            let params = params
                .iter()
                .map(|value| to_sql!(value))
                .collect::<Vec<_>>();
            match self.borrow_mut().execute(&query as &str, &params[..]) {
                Ok(_) => Ok(()),
                Err(e) => Error::new(error_level, "exec error", &e),
            }
        }
    }

    fn iterate_inner<'a>(
        &self,
        query: Cow<'a, str>,
        params: &[Value<'a>],
        error_level: &ErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool,
    ) -> Result<()> {
        let params = params
            .iter()
            .map(|value| to_sql!(value))
            .collect::<Vec<_>>();
        let rows = match self.borrow_mut().query(&query as &str, &params[..]) {
            Ok(result) => result,
            Err(e) => return Error::new(error_level, "exec error", &e),
        };
        let mut pairs = Vec::new();

        for row in rows {
            for (index, col) in row.columns().iter().enumerate() {
                let value = row.get_to_string(index);
                pairs.push((col.name().to_string(), value));
            }
        }

        let pairs: Vec<(&str, Option<&str>)> =
            pairs.iter().map(|p| (&*p.0, p.1.as_deref())).collect();
        if !pairs.is_empty() && !callback(&pairs) {
            return Error::new(error_level, "exec error", "query aborted");
        }

        Ok(())
    }

    fn rows_inner<'a, 'r>(
        &self,
        query: Cow<'a, str>,
        params: &[Value<'a>],
        error_level: &ErrorLevel,
    ) -> Result<Vec<Row<'r>>> {
        let params = params
            .iter()
            .map(|value| to_sql!(value))
            .collect::<Vec<_>>();
        let result = match self.borrow_mut().query(&query as &str, &params[..]) {
            Ok(result) => result,
            Err(e) => return Error::new(error_level, "exec error", &e).map(|_| Vec::new()),
        };
        let mut rows: Vec<Row> = Vec::new();

        // First row
        if let Some(first_row) = result.first() {
            let column_len = first_row.columns().len();
            let columns = first_row
                .columns()
                .iter()
                .map(|col| col.name().to_string())
                .collect();
            let mut row = Row::new(columns);
            for index in 0..column_len {
                unsafe {
                    row.insert(
                        &*(row.column(index) as *const str),
                        first_row.get_to_string(index),
                    );
                }
            }
            rows.push(row);
        }

        // Or later
        for result_row in result.iter().skip(1) {
            let column_len = result_row.columns().len();
            let mut row = Row::new(rows[0].columns());
            for index in 0..column_len {
                unsafe {
                    row.insert(
                        &*(rows[0].column(index) as *const str),
                        result_row.get_to_string(index),
                    );
                }
            }
            rows.push(row);
        }

        Ok(rows)
    }

    fn close(&self) {
        // Do nothing
    }

    #[inline]
    fn kind(&self) -> ConnKind {
        ConnKind::PostgreSQL
    }
}

trait GetToString {
    fn get_to_string(&self, index: usize) -> Option<String>;
}
impl GetToString for postgres::row::Row {
    fn get_to_string(&self, index: usize) -> Option<String> {
        if let Ok(value) = self.try_get::<usize, String>(index) {
            Some(value)
        } else if let Ok(value) = self.try_get::<usize, i32>(index) {
            Some(value.to_string())
        } else if let Ok(value) = self.try_get::<usize, i64>(index) {
            Some(value.to_string())
        } else if let Ok(value) = self.try_get::<usize, u32>(index) {
            Some(value.to_string())
        } else if let Ok(value) = self.try_get::<usize, f32>(index) {
            Some(value.to_string())
        } else if let Ok(value) = self.try_get::<usize, f64>(index) {
            Some(value.to_string())
        } else if let Ok(value) = self.try_get::<usize, bool>(index) {
            Some(value.to_string())
        } else if let Ok(value) = self.try_get::<usize, i8>(index) {
            Some(value.to_string())
        } else if let Ok(value) = self.try_get::<usize, i16>(index) {
            Some(value.to_string())
        } else if let Ok(value) = self.try_get::<usize, std::net::IpAddr>(index) {
            Some(value.to_string())
        } else if let Ok(value) = self.try_get::<usize, Vec<u8>>(index) {
            Some(crate::parser::to_hex(&value))
        } else if let Ok(value) = self.try_get::<usize, Uuid>(index) {
            Some(value.simple().to_string())
        } else if let Ok(value) = self.try_get::<usize, SystemTime>(index) {
            Some(value.to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate as concatsql;
    use concatsql::error::*;
    use concatsql::prelude::*;

    #[test]
    fn open() {
        assert!(crate::postgres::open("postgresql://postgres:postgres@localhost").is_ok());
        assert_eq!(
            crate::postgres::open(""),
            Err(Error::Message(
                "failed to open: invalid configuration: host missing".into()
            ))
        );
        assert_eq!(
            crate::postgres::open("foo\0bar"),
            Err(Error::Message(
                "failed to open: invalid connection string: unexpected EOF".into()
            ))
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
            conn.execute(query!("\0")),
            Err(Error::Message("exec error".into())),
        );
        assert_eq!(
            conn.execute(query!("invalid query")),
            Err(Error::Message("exec error".into())),
        );
        assert!(conn.execute("SELECT 1").is_ok());
    }

    #[test]
    #[cfg(debug_assertions)]
    fn iterate() {
        let conn = crate::postgres::open("postgresql://postgres:postgres@localhost").unwrap();
        assert_eq!(
            conn.iterate(query!("\0"), |_| {
                unreachable!();
            }),
            Err(Error::Message("exec error".into())),
        );
        assert_eq!(
            conn.iterate(query!("invalid query"), |_| {
                unreachable!();
            }),
            Err(Error::Message("exec error".into())),
        );
        assert!(conn.iterate("SELECT 1", |_| { true }).is_ok());
    }
}
