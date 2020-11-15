extern crate sqlite3_sys as ffi;

use std::ffi::{CStr, CString, c_void};
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

    // TODO:
    //     Drop sqlite3_exec, use sqlite3_prepare / sqlite3_step [/ sqlite3_column_bytes / sqlite3_column_blob].
    fn iterate_inner(&self, s: &str, error_level: &ErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>
    {
        let query = match CString::new(s) {
            Ok(string) => string,
            _ => return Error::new(&error_level, "invalid query", s),
        };
        let mut err_msg = ptr::null_mut();
        type F<'a> = &'a mut dyn FnMut(&[(&str, Option<&str>)]) -> bool;

        unsafe {
            ffi::sqlite3_exec(
                self as *const _ as *mut _,
                query.as_ptr(),
                Some(process_callback),
                &callback as *const F as *mut F as *mut c_void,
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

extern "C" fn process_callback(
    callback: *mut c_void,
    count: i32,
    values: *mut *mut i8,
    columns: *mut *mut i8,
) -> i32
{
    type F<'a> = &'a mut dyn FnMut(&[(&str, Option<&'a str>)]) -> bool;
    let mut pairs = Vec::with_capacity(count as usize);
    for i in 0..(count as isize) {
        let column = {
            let pointer = unsafe { *columns.offset(i) };
            debug_assert!(!pointer.is_null());
            std::str::from_utf8(unsafe { CStr::from_ptr(pointer).to_bytes() }).unwrap()
        };
        let value = {
            let pointer = unsafe { *values.offset(i) };
            if pointer.is_null() {
                None
            } else {
                //Some(std::str::from_utf8(unsafe { CStr::from_ptr(pointer).to_bytes() }).unwrap())
                let bytes = unsafe { CStr::from_ptr(pointer).to_bytes() };
                match std::str::from_utf8(&bytes) {
                    Ok(s) => Some(s),
                    Err(_) => {
                        let len = unsafe { ffi::sqlite3_blob_bytes(pointer as *mut ffi::sqlite3_blob) };
                        dbg!(len);
                        //let data = unsafe { ffi::sqlite3_column_blob(columns as *mut ffi::sqlite3_stmt, i as i32) };
                        //dbg!(data);
                        let bytes = unsafe { std::slice::from_raw_parts::<u8>(pointer as *const u8, len as usize)};
                        //pointer = crate::parser::to_hex(bytes).as_ptr() as *mut _;
                        let bytes: &str = Box::leak(crate::parser::to_hex(&bytes).into_boxed_str()); // 'a
                        dbg!(&bytes);
                        //pointer = bytes.as_ptr() as *mut _;
                        //Some(std::str::from_utf8(unsafe { CStr::from_ptr(pointer).to_bytes() }).unwrap())
                        Some(bytes)
                    }
                }
            }
        };
        pairs.push((column, value));
    }
    if unsafe { (*(callback as *mut F))(&pairs) } {
        0
    } else {
        1
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

