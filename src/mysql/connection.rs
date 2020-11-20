extern crate mysql_sys as mysql;
use mysql::{Opts, Conn};
use mysql::prelude::*;

use std::cell::RefCell;
use std::pin::Pin;

use crate::Result;
use crate::parser::to_hex;
use crate::row::{Table, Row};
use crate::connection::{Connection, ConcatsqlConn, ConnKind};
use crate::error::{Error, ErrorLevel};
use crate::wrapstring::{WrapString, Value};

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
        error_level: RefCell::new(ErrorLevel::default()),
    })
}

macro_rules! to_mysql_value {
    ($value:expr) => (
        match $value {
            Value::Null         => mysql::Value::from(None as Option<i32>),
            Value::I32(value)   => mysql::Value::from(value),
            Value::I64(value)   => mysql::Value::from(value),
            Value::I128(value)  => mysql::Value::from(value),
            Value::F32(value)   => mysql::Value::from(value),
            Value::F64(value)   => mysql::Value::from(value),
            Value::Text(value)  => mysql::Value::from(value),
            Value::Bytes(value) => mysql::Value::from(value),
        }
    );
}

impl ConcatsqlConn for RefCell<mysql::Conn> {
    fn execute_inner(&self, ws: &WrapString, error_level: &ErrorLevel) -> Result<()> {
        let mut conn = self.borrow_mut();
        let query = compile(ws);
        if ws.params.is_empty() {
            match conn.query_drop(&query) {
                Ok(_) => Ok(()),
                Err(e) => Error::new(error_level, "exec error", &e),
            }
        } else {
            let params = ws.params.iter().map(|value| to_mysql_value!(value)).collect::<Vec<_>>();
            match conn.exec_drop(&query, params) {
                Ok(_) => Ok(()),
                Err(e) => Error::new(error_level, "exec error", &e),
            }
        }
    }

    fn iterate_inner(&self, ws: &WrapString, error_level: &ErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>
    {
        macro_rules! run {
            ($result:expr) => {
                while let Some(result_set) = $result.next_set() {
                    let result_set = match result_set {
                        Ok(result_set) => result_set,
                        Err(e) => return Error::new(error_level, "exec error", &e),
                    };
                    let mut pairs: Vec<(String, Option<String>)> = Vec::with_capacity(result_set.affected_rows() as usize);

                    for row in result_set {
                        let row = match row {
                            Ok(row) => row,
                            Err(e) => return Error::new(error_level, "exec error", &e),
                        };

                        for (index, col) in row.columns().iter().enumerate() {
                            let value = row.get_to_string(index);
                            pairs.push((col.name_str().to_string(), value));
                        }
                    }

                    let pairs: Vec<(&str, Option<&str>)> = pairs.iter().map(|p| (&*p.0, p.1.as_deref())).collect();
                    if !pairs.is_empty() && !callback(&pairs) {
                        return Error::new(error_level, "exec error", "query aborted");
                    }
                }
            };
        }

        let mut conn = self.borrow_mut();
        let query = compile(ws);

        if ws.params.is_empty() {
            let mut result = match conn.query_iter(&query) {
                Ok(result) => result,
                Err(e) => return Error::new(error_level, "exec error", &e),
            };
            run!(result);
        } else {
            let params = ws.params.iter().map(|value| to_mysql_value!(value)).collect::<Vec<_>>();
            let mut result = match conn.exec_iter(&query, params) {
                Ok(result) => result,
                Err(e) => return Error::new(error_level, "exec error", &e),
            };
            run!(result);
        }

        Ok(())
    }

    fn rows_inner<'a>(&self, ws: &WrapString, error_level: &ErrorLevel) -> Result<Table<'a>> {
        let mut conn = self.borrow_mut();
        let query = compile(ws);

        macro_rules! run {
            ($result:expr, $table:expr) => {
                if let Some(result_set) = $result.next_set() {
                    let result_set = match result_set {
                        Ok(result_set) => result_set,
                        Err(e) => return Error::new(error_level, "exec error", &e).map(|_|Table::default()),
                    };

                    let mut first_row = true;

                    for result_row in result_set {
                        let result_row = match result_row {
                            Ok(row) => row,
                            Err(e) => return Error::new(error_level, "exec error", &e).map(|_|Table::default()),
                        };

                        let column_len = result_row.columns_ref().len();
                        let mut row = Row::with_capacity(column_len);

                        if first_row {
                            first_row = false;
                            for (index, col) in result_row.columns_ref().iter().enumerate() {
                                let value = result_row.get_to_string(index);
                                let column = Box::leak(col.name_str().to_string().into_boxed_str());
                                $table.push_column(column);
                                row.insert(&*column, value);
                            }
                        } else {
                            for index in 0..column_len {
                                let value = result_row.get_to_string(index);
                                unsafe {
                                    row.insert(&*$table.column_names[index], value);
                                }
                            }
                        }

                        $table.push(row);
                    }
                }
            };
        }

        let mut table = Table::default();

        if ws.params.is_empty() {
            let mut result = match conn.query_iter(&query) {
                Ok(result) => result,
                Err(e) => return Error::new(error_level, "exec error", &e).map(|_|Table::default()),
            };
            run!(result, table);
        } else {
            let params = ws.params.iter().map(|value| to_mysql_value!(value)).collect::<Vec<_>>();
            let mut result = match conn.exec_iter(&query, params) {
                Ok(result) => result,
                Err(e) => return Error::new(error_level, "exec error", &e).map(|_|Table::default()),
            };
            run!(result, table);
        }

        Ok(table)
    }

    fn kind(&self) -> ConnKind {
        ConnKind::MySQL
    }
}

fn compile(ws: &WrapString) -> String {
    let mut query = String::new();
    for part in &ws.query {
        match part {
            Some(s) => query.push_str(&s),
            None =>    query.push('?'),
        }
    }
    query
}

trait GetToString {
    fn get_to_string(&self, index: usize) -> Option<String>;
}
impl GetToString for mysql::Row {
    fn get_to_string(&self, index: usize) -> Option<String> {
        match self[index] {
            mysql::Value::NULL      => None,
            mysql::Value::Int(v)    => Some(v.to_string()),
            mysql::Value::UInt(v)   => Some(v.to_string()),
            mysql::Value::Float(v)  => Some(v.to_string()),
            mysql::Value::Double(v) => Some(v.to_string()),
            mysql::Value::Bytes(ref bytes) => match String::from_utf8(bytes.to_vec()) {
                Ok(string) => Some(string),
                Err(_) => Some(to_hex(&bytes)),
            }
            _ => unimplemented!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate as concatsql;
    use concatsql::error::*;
    #[cfg(debug_assertions)]
    use concatsql::prelude::*;

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
        assert!(conn.execute("SELECT 1").is_ok());
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
        assert!(conn.iterate("SELECT 1", |_|{true}).is_ok());
    }
}
