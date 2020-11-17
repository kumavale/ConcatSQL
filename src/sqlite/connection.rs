extern crate sqlite3_sys as ffi;

use std::ffi::{CStr, CString};
use std::ptr;
use std::path::Path;
use std::pin::Pin;
use std::cell::RefCell;

use crate::Result;
use crate::row::Row;
use crate::connection::{Connection, ConcatsqlConn, ConnKind};
use crate::error::{Error, ErrorLevel};

/// Open a read-write connection to a new or existing database.
pub fn open<'a, T: AsRef<Path>>(path: T, openflags: i32) -> Result<Connection<'a>> {
    let path = match path.as_ref().to_str() {
        Some(path) => {
            match CString::new(path) {
                Ok(string) => string,
                _ => return Err(Error::Message(format!("invalid path: {}", path))),
            }
        },
        _ => return Err(Error::Message(format!("failed to open path: {:?}", path.as_ref()))),
    };
    let mut conn_ptr = ptr::null_mut();

    let open_result = unsafe { ffi::sqlite3_open_v2(
        path.as_ptr(),
        &mut conn_ptr,
        openflags,
        ptr::null())
    };

    match open_result {
        ffi::SQLITE_OK =>
            Ok(Connection {
                conn:        unsafe { Pin::new_unchecked(&*conn_ptr) },
                error_level: RefCell::new(ErrorLevel::default()),
            }),
        _ => Err(Error::Message("failed to connect".into())),
    }
}

impl ConcatsqlConn for ffi::sqlite3 {
    fn execute_inner(&self, s: &str, error_level: &ErrorLevel) -> Result<()> {
        let query = match CString::new(s) {
            Ok(string) => string,
            _ => return Error::new(&error_level, "invalid query", s),
        };
        let mut err_msg = ptr::null_mut();

        unsafe {
            ffi::sqlite3_exec(
                self as *const _ as *mut _,
                query.as_ptr(),
                None,             // callback fn
                ptr::null_mut(),  // callback arg
                &mut err_msg,
            );
        }

        if err_msg.is_null() {
            Ok(())
        } else {
            Error::new(&error_level, "exec error",
                unsafe{ &CStr::from_ptr(ffi::sqlite3_errmsg(self as *const _ as *mut _)).to_string_lossy() })
        }
    }

    fn iterate_inner(&self, s: &str, error_level: &ErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>
    {
        let query = match CString::new(s) {
            Ok(string) => string,
            _ => return Error::new(&error_level, "invalid query", s),
        };
        let mut stmt = ptr::null_mut();

        unsafe {
            let result = ffi::sqlite3_prepare_v2(
                self as *const _ as *mut _,
                query.as_ptr(),
                -1,
                &mut stmt,
                ptr::null_mut(),
            );

            if result != ffi::SQLITE_OK {
                ffi::sqlite3_finalize(stmt);
                return Error::new(&error_level, "exec error",
                    &CStr::from_ptr(ffi::sqlite3_errmsg(self as *const _ as *mut _)).to_string_lossy());
            }

            let column_count = ffi::sqlite3_column_count(stmt) as i32;

            loop {
                match ffi::sqlite3_step(stmt) {
                    ffi::SQLITE_DONE => break,
                    ffi::SQLITE_ROW => {
                        let mut pairs = Vec::with_capacity(column_count as usize);
                        pairs.storing(stmt, column_count);
                        if !callback(&pairs) {
                            break;
                        }
                    }
                    _ => {
                        ffi::sqlite3_finalize(stmt);
                        return Error::new(&error_level, "exec error",
                            &CStr::from_ptr(ffi::sqlite3_errmsg(self as *const _ as *mut _)).to_string_lossy());
                    }
                }
            }

            ffi::sqlite3_finalize(stmt);
            Ok(())
        }
    }

    fn rows_inner(&self, query: &str, error_level: &ErrorLevel) -> Result<Vec<Row>> {
        let mut rows: Vec<Row> = Vec::new();

        self.iterate_inner(query, error_level, &mut |pairs: &[(&str, Option<&str>)]| {
            let mut row = Row::new();
            for (column, value) in pairs.iter() {
                row.insert(column.to_string(), value.map(|v| v.to_string()));
            }
            rows.push(row);
            true
        })?;

        Ok(rows)
    }

    fn kind(&self) -> ConnKind {
        ConnKind::SQLite
    }
}

trait Storing {
    unsafe fn storing(&mut self, stmt: *mut ffi::sqlite3_stmt, column_count: i32);
}
impl Storing for Vec<(&str, Option<&str>)> {
    unsafe fn storing(&mut self, stmt: *mut ffi::sqlite3_stmt, column_count: i32) {
        for i in 0..(column_count) {
            let column_name = {
                let column_name = ffi::sqlite3_column_name(stmt, i);
                std::str::from_utf8(CStr::from_ptr(column_name).to_bytes()).unwrap()
            };
            let value = {
                match ffi::sqlite3_column_type(stmt, i) {
                    ffi::SQLITE_BLOB => {
                        let ptr = ffi::sqlite3_column_blob(stmt, i);
                        let count = ffi::sqlite3_column_bytes(stmt, i) as usize;
                        let bytes = std::slice::from_raw_parts::<u8>(ptr as *const u8, count);
                        Some(Box::leak(crate::parser::to_hex(&bytes).into_boxed_str()) as &str)
                    },
                    ffi::SQLITE_INTEGER |
                    ffi::SQLITE_FLOAT   |
                    ffi::SQLITE_TEXT    => {
                        let ptr = ffi::sqlite3_column_text(stmt, i) as *const i8;
                        Some(std::str::from_utf8(CStr::from_ptr(ptr).to_bytes()).unwrap())
                    }
                    _  /* ffi::SQLITE_NULL */ => None
                }
            };
            self.push((column_name, value));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate as concatsql;
    use concatsql::error::*;
    use temporary::Directory;
    #[cfg(debug_assertions)]
    use concatsql::prelude::*;

    #[test]
    fn open() {
        let dir = Directory::new("sqlite").unwrap();
        let path = dir.path().join("test.db");
        assert_ne!(crate::sqlite::open(""), crate::sqlite::open(""));
        assert_ne!(crate::sqlite::open(":memory:"), crate::sqlite::open(":memory:"));
        assert_ne!(crate::sqlite::open(&path), crate::sqlite::open(&path));
        assert_eq!(
            crate::sqlite::open("foo\0bar"),
            Err(Error::Message("invalid path: foo\u{0}bar".into()))
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    fn execute() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(
            conn.execute(prep!("\0")),
            Err(Error::Message("invalid query".into())),
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
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(
            conn.iterate(prep!("\0"), |_| { unreachable!(); }),
            Err(Error::Message("invalid query".into())),
        );
        assert_eq!(
            conn.iterate(prep!("invalid query"), |_| { unreachable!(); }),
            Err(Error::Message("exec error".into())),
        );
        assert!(conn.iterate("SELECT 1", |_|{true}).is_ok());
    }

    #[test]
    #[cfg(debug_assertions)]
    fn actual_sql() {
        assert_eq!(prep!("SELECT").actual_sql(), "SELECT");
        assert_eq!(prep!("O''Reilly").actual_sql(), "O''Reilly");
        assert_eq!(prep!("\"O'Reilly\"").actual_sql(), "\"O'Reilly\"");

        //crate::prep!("O'Reilly").actual_sql();      // panic
    }

    #[test]
    fn debug_display() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(format!("{:?}", &conn), format!("{:?}", &conn));
    }
}

