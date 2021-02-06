extern crate mysql_sys as mysql;
use mysql::{Opts, Conn};
use mysql::prelude::*;

use std::cell::{Cell, RefCell};
use std::borrow::Cow;

use crate::Result;
use crate::parser::to_hex;
use crate::row::Row;
use crate::connection::{Connection, ConcatsqlConn, ConnKind};
use crate::error::{Error, ErrorLevel};
use crate::value::{Value, SystemTimeToString};

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
        conn:        Box::new(RefCell::new(conn)),
        error_level: Cell::new(ErrorLevel::default()),
    })
}

macro_rules! to_mysql_value {
    ($value:expr) => (
        match $value {
            Value::Null          => mysql::Value::from(None as Option<i32>),
            Value::I32(value)    => mysql::Value::from(value),
            Value::I64(value)    => mysql::Value::from(value),
            Value::F32(value)    => mysql::Value::from(value),
            Value::F64(value)    => mysql::Value::from(value),
            Value::Text(value)   => mysql::Value::from(value.as_ref()),
            Value::Bytes(value)  => mysql::Value::from(value),
            Value::IpAddr(value) => mysql::Value::from(value.to_string()),
            Value::Time(value)   => mysql::Value::from(value.to_string()),
        }
    );
}

impl ConcatsqlConn for RefCell<mysql::Conn> {
    fn execute_inner<'a>(&self, query: Cow<'a, str>, params: &[Value<'a>], error_level: &ErrorLevel) -> Result<()> {
        let mut conn = self.borrow_mut();
        if params.is_empty() {
            match conn.query_drop(&query) {
                Ok(_) => Ok(()),
                Err(e) => Error::new(error_level, "exec error", &e),
            }
        } else {
            let params = params.iter().map(|value| to_mysql_value!(value)).collect::<Vec<_>>();
            match conn.exec_drop(&query, params) {
                Ok(_) => Ok(()),
                Err(e) => Error::new(error_level, "exec error", &e),
            }
        }
    }

    fn iterate_inner<'a>(&self, query: Cow<'a, str>, params: &[Value<'a>], error_level: &ErrorLevel,
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

        if params.is_empty() {
            let mut result = match conn.query_iter(&query) {
                Ok(result) => result,
                Err(e) => return Error::new(error_level, "exec error", &e),
            };
            run!(result);
        } else {
            let params = params.iter().map(|value| to_mysql_value!(value)).collect::<Vec<_>>();
            let mut result = match conn.exec_iter(&query, params) {
                Ok(result) => result,
                Err(e) => return Error::new(error_level, "exec error", &e),
            };
            run!(result);
        }

        Ok(())
    }

    fn rows_inner<'a, 'r>(&self, query: Cow<'a, str>, params: &[Value<'a>], error_level: &ErrorLevel)
        -> Result<Vec<Row<'r>>>
    {
        let mut conn = self.borrow_mut();

        macro_rules! run {
            ($result:expr, $rows:expr) => {
                if let Some(result_set) = $result.next_set() {
                    let result_set = match result_set {
                        Ok(result_set) => result_set,
                        Err(e) => return Error::new(error_level, "exec error", &e).map(|_| Vec::new()),
                    };

                    let mut first_row = true;

                    for result_row in result_set {
                        let result_row = match result_row {
                            Ok(row) => row,
                            Err(e) => return Error::new(error_level, "exec error", &e).map(|_| Vec::new()),
                        };

                        let column_len = result_row.columns_ref().len();

                        if first_row {
                            first_row = false;
                            let columns = result_row.columns_ref().iter().map(|col|col.name_str().to_string()).collect();
                            let mut row = Row::new(columns);
                            for index in 0..column_len {
                                unsafe {
                                    row.insert(&*(row.column(index) as *const str), result_row.get_to_string(index));
                                }
                            }
                            $rows.push(row);
                        } else {
                            let mut row = Row::new($rows[0].columns());
                            for index in 0..column_len {
                                unsafe {
                                    row.insert(&*($rows[0].column(index) as *const str), result_row.get_to_string(index));
                                }
                            }
                            $rows.push(row);
                        }
                    }
                }
            };
        }

        let mut rows: Vec<Row> = Vec::new();

        if params.is_empty() {
            let mut result = match conn.query_iter(&query) {
                Ok(result) => result,
                Err(e) => return Error::new(error_level, "exec error", &e).map(|_| Vec::new()),
            };
            run!(result, rows);
        } else {
            let params = params.iter().map(|value| to_mysql_value!(value)).collect::<Vec<_>>();
            let mut result = match conn.exec_iter(&query, params) {
                Ok(result) => result,
                Err(e) => return Error::new(error_level, "exec error", &e).map(|_| Vec::new()),
            };
            run!(result, rows);
        }

        Ok(rows)
    }

    fn close(&self) {
        // Do nothing
    }

    #[inline]
    fn kind(&self) -> ConnKind {
        ConnKind::MySQL
    }
}

trait GetToString {
    fn get_to_string(&self, index: usize) -> Option<String>;
}
impl GetToString for mysql::Row {
    fn get_to_string(&self, index: usize) -> Option<String> {
        match self[index] {
            mysql::Value::NULL      => None,
            mysql::Value::Int(v)    => Some(v.to_string()),
            mysql::Value::UInt(v)   => Some(v.to_string()),  // unreachable ?
            mysql::Value::Float(v)  => Some(v.to_string()),  // unreachable ?
            mysql::Value::Double(v) => Some(v.to_string()),  // unreachable ?
            mysql::Value::Bytes(ref bytes) => match String::from_utf8(bytes.to_vec()) {
                Ok(string) => Some(string),
                Err(_) => Some(to_hex(&bytes)),
            }
            mysql::Value::Date(year, month, day, hour, minute, second, micros) => Some(format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}", year, month, day, hour, minute, second, micros
            )),  // unreachable ?
            mysql::Value::Time(neg, days, hours, minutes, seconds, micros) => {
                Some(if neg {
                    format!(
                        "-{:03}:{:02}:{:02}.{:06}",
                        days * 24 + u32::from(hours),
                        minutes,
                        seconds,
                        micros
                    )
                } else {
                    format!(
                        "{:03}:{:02}:{:02}.{:06}",
                        days * 24 + u32::from(hours),
                        minutes,
                        seconds,
                        micros
                    )
                })
            }  // unreachable ?
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

    #[test]
    fn get_to_string() {
        let conn = crate::mysql::open("mysql://localhost:3306/test").unwrap();
        #[cfg(debug_assertions)] conn.error_level(ErrorLevel::Debug);
        conn.execute("
            CREATE TEMPORARY TABLE test (bytes BLOB, i32 INT, f32 FLOAT, f64 DOUBLE, date DATE, time TIME, none INT);
            INSERT INTO test(bytes, i32, f32, f64, date, time) VALUES(X'ABCD', 1, 2, 3, '1900-01-01', '123:00:00');
        ").unwrap();
        assert_eq!(conn.rows("SELECT bytes FROM test").unwrap().first().unwrap().get(0).unwrap(), "ABCD");
        assert_eq!(conn.rows("SELECT   i32 FROM test").unwrap().first().unwrap().get(0).unwrap(), "1");
        assert_eq!(conn.rows("SELECT   f32 FROM test").unwrap().first().unwrap().get(0).unwrap(), "2");
        assert_eq!(conn.rows("SELECT   f64 FROM test").unwrap().first().unwrap().get(0).unwrap(), "3");
        assert_eq!(conn.rows("SELECT  date FROM test").unwrap().first().unwrap().get(0).unwrap(), "1900-01-01");
        assert_eq!(conn.rows("SELECT  time FROM test").unwrap().first().unwrap().get(0).unwrap(), "123:00:00");
    }
}
