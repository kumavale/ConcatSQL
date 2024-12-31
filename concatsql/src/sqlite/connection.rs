extern crate sqlite3_sys as ffi;

use std::ffi::{CStr, CString, c_void};
use std::ptr::{self, NonNull};
use std::path::Path;
use std::cell::Cell;
use std::borrow::Cow;

use crate::Result;
use crate::row::Row;
use crate::connection::{Connection, ConcatsqlConn, ConnKind};
use crate::error::{Error, ErrorLevel};
use crate::value::{Value, SystemTimeToString};

/// Open a read-write connection to a new or existing database.
pub fn open<T: AsRef<Path>>(path: T, openflags: i32) -> Result<Connection> {
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
                conn:        Box::new(unsafe { NonNull::new_unchecked(conn_ptr) }),
                error_level: Cell::new(ErrorLevel::default()),
            }),
        _ => {
            unsafe { ffi::sqlite3_close(conn_ptr); }
            Err(Error::Message("failed to connect".into()))
        }
    }
}

impl ConcatsqlConn for NonNull<ffi::sqlite3> {
    fn execute_inner<'a>(&self, query: Cow<'a, str>, params: &[Value<'a>], error_level: &ErrorLevel) -> Result<()> {
        let query = match CString::new(query.as_bytes()) {
            Ok(string) => string,
            _ => return Error::new(error_level, "invalid query", query),
        };

        let mut stmt = ptr::null_mut();

        if params.is_empty() {
            let mut errmsg = ptr::null_mut();
            unsafe {
                ffi::sqlite3_exec(
                    self.as_ptr(),
                    query.as_ptr(),
                    None,             // callback fn
                    ptr::null_mut(),  // callback arg
                    &mut errmsg,
                );
            }

            if errmsg.is_null() {
                return Ok(());
            } else {
                unsafe {
                    ffi::sqlite3_finalize(stmt);
                    ffi::sqlite3_free(errmsg as *mut _);
                    return Error::new(error_level, "exec error",
                        CStr::from_ptr(ffi::sqlite3_errmsg(self.as_ptr())).to_string_lossy());
                }
            }
        }

        unsafe {
            let result = ffi::sqlite3_prepare_v2(
                self.as_ptr(),
                query.as_ptr(),
                -1,
                &mut stmt,
                ptr::null_mut(),
            );

            if result != ffi::SQLITE_OK {
                ffi::sqlite3_finalize(stmt);
                return Error::new(error_level, "exec error",
                    CStr::from_ptr(ffi::sqlite3_errmsg(self.as_ptr())).to_string_lossy());
            }

            bind_all(stmt, params, error_level)?;

            loop {
                match ffi::sqlite3_step(stmt) {
                    ffi::SQLITE_DONE => break,
                    ffi::SQLITE_ROW => (),  // Do nothing
                    _ => {
                        ffi::sqlite3_finalize(stmt);
                        return Error::new(error_level, "exec error",
                            CStr::from_ptr(ffi::sqlite3_errmsg(self.as_ptr())).to_string_lossy());
                    }
                }
            }

            ffi::sqlite3_finalize(stmt);
            Ok(())
        }
    }

    fn iterate_inner<'a>(&self, query: Cow<'a, str>, params: &[Value<'a>], error_level: &ErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>
    {
        let query = match CString::new(query.as_bytes()) {
            Ok(string) => string,
            _ => return Error::new(error_level, "invalid query", query),
        };
        let mut stmt = ptr::null_mut();

        unsafe {
            let result = ffi::sqlite3_prepare_v2(
                self.as_ptr(),
                query.as_ptr(),
                -1,
                &mut stmt,
                ptr::null_mut(),
            );

            if result != ffi::SQLITE_OK {
                ffi::sqlite3_finalize(stmt);
                return Error::new(error_level, "exec error",
                    CStr::from_ptr(ffi::sqlite3_errmsg(self.as_ptr())).to_string_lossy());
            }

            bind_all(stmt, params, error_level)?;

            let column_count = ffi::sqlite3_column_count(stmt);

            loop {
                match ffi::sqlite3_step(stmt) {
                    ffi::SQLITE_DONE => break,
                    ffi::SQLITE_ROW => {
                        let mut pairs = Vec::with_capacity(column_count as usize);
                        pairs.storing(stmt, column_count);
                        let pairs: Vec<(&str, Option<&str>)> = pairs.iter().map(|p| (p.0, p.1.as_deref())).collect();
                        if !callback(&pairs) {
                            break;
                        }
                    }
                    _ => {
                        ffi::sqlite3_finalize(stmt);
                        return Error::new(error_level, "exec error",
                            CStr::from_ptr(ffi::sqlite3_errmsg(self.as_ptr())).to_string_lossy());
                    }
                }
            }

            ffi::sqlite3_finalize(stmt);
            Ok(())
        }
    }

    fn rows_inner<'a, 'r>(&self, query: Cow<'a, str>, params: &[Value<'a>], error_level: &ErrorLevel)
        -> Result<Vec<Row<'r>>>
    {
        let mut rows: Vec<Row> = Vec::new();
        let query = match CString::new(query.as_bytes()) {
            Ok(string) => string,
            _ => return Error::new(error_level, "invalid query", query).map(|_| Vec::new()),
        };
        let mut stmt = ptr::null_mut();

        unsafe {
            let result = ffi::sqlite3_prepare_v2(
                self.as_ptr(),
                query.as_ptr(),
                -1,
                &mut stmt,
                ptr::null_mut(),
            );

            if result != ffi::SQLITE_OK {
                ffi::sqlite3_finalize(stmt);
                return Error::new(error_level, "exec error",
                    CStr::from_ptr(ffi::sqlite3_errmsg(self.as_ptr())).to_string_lossy())
                    .map(|_| Vec::new());
            }

            bind_all(stmt, params, error_level)?;

            let column_count = ffi::sqlite3_column_count(stmt);

            // First row
            match ffi::sqlite3_step(stmt) {
                ffi::SQLITE_DONE => {
                    ffi::sqlite3_finalize(stmt);
                    return Ok(rows);
                }
                ffi::SQLITE_ROW => {
                    let mut pairs = Vec::with_capacity(column_count as usize);
                    pairs.storing(stmt, column_count);
                    let columns = pairs.iter().map(|(column, _)|column.to_string()).collect();
                    let mut row = Row::new(columns);
                    for (index, (_, value)) in pairs.into_iter().enumerate() {
                        row.insert(&*(row.column(index) as *const str), value);
                    }
                    rows.push(row);
                }
                _ => {
                    ffi::sqlite3_finalize(stmt);
                    return Error::new(error_level, "exec error",
                        CStr::from_ptr(ffi::sqlite3_errmsg(self.as_ptr())).to_string_lossy())
                        .map(|_| Vec::new());
                }
            }

            // Or later
            loop {
                match ffi::sqlite3_step(stmt) {
                    ffi::SQLITE_DONE => break,
                    ffi::SQLITE_ROW => {
                        let mut pairs = Vec::with_capacity(column_count as usize);
                        pairs.storing(stmt, column_count);
                        let mut row = Row::new(rows[0].columns());
                        for (index, (_, value)) in pairs.into_iter().enumerate() {
                            row.insert(&*(rows[0].column(index) as *const str), value);
                        }
                        rows.push(row);
                    }
                    _ => {
                        ffi::sqlite3_finalize(stmt);
                        return Error::new(error_level, "exec error",
                            CStr::from_ptr(ffi::sqlite3_errmsg(self.as_ptr())).to_string_lossy())
                            .map(|_| Vec::new());
                    }
                }
            }

            ffi::sqlite3_finalize(stmt);
            Ok(rows)
        }
    }

    fn close(&self) {
        unsafe {
            ffi::sqlite3_busy_handler(self.as_ptr() as *const _ as *mut ffi::sqlite3, None, std::ptr::null_mut());
            let close_result = ffi::sqlite3_close(self.as_ptr() as *const _ as *mut ffi::sqlite3);
            std::ptr::drop_in_place(self.as_ptr() as *const _ as *mut ffi::sqlite3);
            if close_result != ffi::SQLITE_OK {
                eprintln!("error closing SQLite connection: {}", close_result);
            }
        }
    }

    #[inline]
    fn kind(&self) -> ConnKind {
        ConnKind::SQLite
    }
}

trait Storing {
    unsafe fn storing(&mut self, stmt: *mut ffi::sqlite3_stmt, column_count: i32);
}
impl Storing for Vec<(&str, Option<String>)> {
    unsafe fn storing(&mut self, stmt: *mut ffi::sqlite3_stmt, column_count: i32) {
        for i in 0..column_count {
            let column_name = {
                let column_name = ffi::sqlite3_column_name(stmt, i);
                CStr::from_ptr(column_name).to_str().unwrap()
            };
            let value = {
                match ffi::sqlite3_column_type(stmt, i) {
                    ffi::SQLITE_BLOB => {
                        let ptr = ffi::sqlite3_column_blob(stmt, i);
                        let count = ffi::sqlite3_column_bytes(stmt, i) as usize;
                        let bytes = std::slice::from_raw_parts::<u8>(ptr as *const u8, count);
                        Some(crate::parser::to_hex(bytes))
                    },
                    ffi::SQLITE_INTEGER |
                    ffi::SQLITE_FLOAT   |
                    ffi::SQLITE_TEXT    => {
                        let ptr = ffi::sqlite3_column_text(stmt, i) as *const _ as *mut i8;
                        Some(CStr::from_ptr(ptr).to_str().unwrap().to_owned())
                    }
                    _  /* ffi::SQLITE_NULL */ => None
                }
            };
            self.push((column_name, value));
        }
    }
}

unsafe fn bind_all(stmt: *mut ffi::sqlite3_stmt, params: &[Value<'_>], error_level: &ErrorLevel) -> Result<()> {
    for (index, param) in (1i32..).zip(params.iter()) {
        let result = match param {
            Value::Null => {
                ffi::sqlite3_bind_null(stmt, index)
            }
            Value::I32(value) => {
                ffi::sqlite3_bind_int(stmt, index, *value)
            }
            Value::I64(value) => {
                ffi::sqlite3_bind_int64(stmt, index, *value)
            }
            Value::F32(value) => {
                ffi::sqlite3_bind_double(stmt, index, *value as f64)
            }
            Value::F64(value) => {
                ffi::sqlite3_bind_double(stmt, index, *value)
            }
            Value::Text(value) => {
                ffi::sqlite3_bind_text(
                    stmt,
                    index,
                    value.as_ptr() as *const _,
                    value.len() as i32,
                    Some(std::mem::transmute::<*const c_void, extern "C" fn(*mut c_void)>(ffi::SQLITE_TRANSIENT as *const c_void)),
                )
            }
            Value::Bytes(value) => {
                ffi::sqlite3_bind_blob(
                    stmt,
                    index,
                    value.as_ptr() as *const _,
                    value.len() as i32,
                    Some(std::mem::transmute::<*const c_void, extern "C" fn(*mut c_void)>(ffi::SQLITE_TRANSIENT as *const c_void)),
                )
            }
            Value::IpAddr(value) => {
                let value = value.to_string();
                ffi::sqlite3_bind_text(
                    stmt,
                    index,
                    value.as_ptr() as *const _,
                    value.len() as i32,
                    Some(std::mem::transmute::<*const c_void, extern "C" fn(*mut c_void)>(ffi::SQLITE_TRANSIENT as *const c_void)),
                )
            }
            Value::Time(value) => {
                let value = value.to_string();
                ffi::sqlite3_bind_text(
                    stmt,
                    index,
                    value.as_ptr() as *const _,
                    value.len() as i32,
                    Some(std::mem::transmute::<*const c_void, extern "C" fn(*mut c_void)>(ffi::SQLITE_TRANSIENT as *const c_void)),
                )
            }
        };
        if result != ffi::SQLITE_OK {
            ffi::sqlite3_finalize(stmt);
            return Error::new(error_level, "bind error", CStr::from_ptr(ffi::sqlite3_errstr(result)).to_string_lossy());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate as concatsql;
    use concatsql::error::*;
    use temporary::Directory;
    #[cfg(debug_assertions)]
    use concatsql::prep;

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
    fn simulate() {
        assert_eq!(prep!("SELECT").simulate(), "SELECT");
        assert_eq!(prep!("O''Reilly").simulate(), "O''Reilly");
        assert_eq!(prep!("\"O'Reilly\"").simulate(), "\"O'Reilly\"");
        assert_eq!(prep!("O'Reilly").simulate(), "O'Reilly");
    }

    #[test]
    fn debug_display() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(format!("{:?}", &conn), format!("{:?}", &conn));
    }
}

