extern crate postgres_sys as postgres;

use postgres::{Client, NoTls};

use std::cell::RefCell;
use std::pin::Pin;

use crate::Result;
use crate::row::Row;
use crate::connection::{Connection, ConcatsqlConn, ConnKind};
use crate::error::{Error, ErrorLevel};
use crate::wrapstring::{WrapString, Value};

/// Open a read-write connection to a new or existing database.
pub fn open(params: &str) -> Result<Connection> {
    let conn = match Client::connect(&params, NoTls) {
        Ok(conn) => conn,
        Err(e) => return Err(Error::Message(format!("failed to open: {}", e))),
    };

    Ok(Connection {
        conn:        unsafe { Pin::new_unchecked(&*Box::leak(Box::new(RefCell::new(conn)))) },
        error_level: RefCell::new(ErrorLevel::default()),
    })
}

macro_rules! to_sql {
    ($value:expr) => (
        match $value {
            Value::Null         => &"NULL" as &(dyn postgres::types::ToSql + Sync),
            Value::I32(value)   => value,
            Value::I64(value)   => value,
            Value::I128(_value) => unimplemented!(),
            Value::F32(value)   => value,
            Value::F64(value)   => value,
            Value::Text(value)  => value,
            Value::Bytes(value) => value,
        }
    );
}

impl ConcatsqlConn for RefCell<postgres::Client> {
    fn execute_inner(&self, ws: &WrapString, error_level: &ErrorLevel) -> Result<()> {
        let query = compile(ws);
        if ws.params.is_empty() {
            match self.borrow_mut().batch_execute(&query) {
                Ok(_) => Ok(()),
                Err(e) => Error::new(error_level, "exec error", &e),
            }
        } else {
            let params = ws.params.iter().map(|value| to_sql!(value)).collect::<Vec<_>>();
            match self.borrow_mut().execute(&query as &str, &params[..]) {
                Ok(_) => Ok(()),
                Err(e) => Error::new(error_level, "exec error", &e),
            }
        }
    }

    fn iterate_inner(&self, ws: &WrapString, error_level: &ErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>
    {
        let query = compile(ws);
        let params = ws.params.iter().map(|value| to_sql!(value)).collect::<Vec<_>>();
        let rows = match self.borrow_mut().query(&query as &str, &params[..]) {
            Ok(result) => result,
            Err(e) => return Error::new(error_level, "exec error", &e),
        };

        let mut pairs = Vec::new();
        for row in rows {
            for (i, col) in row.columns().iter().enumerate() {
                let value = if let Ok(value) = row.try_get::<usize, String>(i) {
                    Some(value)
                } else if let Ok(value) = row.try_get::<usize, i32>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, i64>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, u32>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, f32>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, f64>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, bool>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, i8>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, i16>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, std::net::IpAddr>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = row.try_get::<usize, Vec<u8>>(i) {
                    Some(crate::parser::to_hex(&value))
                } else {
                    None
                };

                pairs.push((col.name().to_string(), value));
            }
        }

        let pairs: Vec<(&str, Option<&str>)> = pairs.iter().map(|p| (&*p.0, p.1.as_deref())).collect();
        if !pairs.is_empty() && !callback(&pairs) {
            return Error::new(error_level, "exec error", "query aborted");
        }

        Ok(())
    }

    fn rows_inner(&self, ws: &WrapString, error_level: &ErrorLevel) -> Result<Vec<Row>> {
        let query = compile(ws);
        let params = ws.params.iter().map(|value| to_sql!(value)).collect::<Vec<_>>();
        let result = match self.borrow_mut().query(&query as &str, &params[..]) {
            Ok(result) => result,
            Err(e) => return Error::new(error_level, "exec error", &e).map(|_|Vec::new()),
        };

        let mut rows: Vec<Row> = Vec::new();

        for result_row in result {
            let mut row = Row::new();

            for (i, col) in result_row.columns().iter().enumerate() {
                let value = if let Ok(value) = result_row.try_get::<usize, String>(i) {
                    Some(value)
                } else if let Ok(value) = result_row.try_get::<usize, i32>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, i64>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, u32>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, f32>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, f64>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, bool>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, i8>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, i16>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, std::net::IpAddr>(i) {
                    Some(value.to_string())
                } else if let Ok(value) = result_row.try_get::<usize, Vec<u8>>(i) {
                    Some(crate::parser::to_hex(&value))
                } else {
                    None
                };

                row.insert(col.name().to_string(), value);
            }
            rows.push(row);
        }

        Ok(rows)
    }

    fn kind(&self) -> ConnKind {
        ConnKind::PostgreSQL
    }
}

fn compile(ws: &WrapString) -> String {
    let mut query = String::new();
    let mut index = 1;
    for part in &ws.query {
        match part {
            Some(s) => query.push_str(&s),
            None => {
                query.push_str(&format!("${}", index));
                index += 1;
            }
        }
    }
    query
}


#[cfg(test)]
mod tests {
    use crate as concatsql;
    use concatsql::error::*;
    #[cfg(debug_assertions)]
    use concatsql::prelude::*;

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
        assert!(conn.execute("SELECT 1").is_ok());
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
        assert!(conn.iterate("SELECT 1", |_|{true}).is_ok());
    }
}
