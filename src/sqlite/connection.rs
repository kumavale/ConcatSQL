extern crate sqlite3_sys as ffi;

use std::ffi::{CStr, CString, c_void};
use std::ptr::{self, NonNull};
use std::path::Path;

use crate::Result;
use crate::connection::{Connection, ConcatsqlConn};
use crate::error::{Error, ErrorLevel};
use crate::wrapstring::WrapString;

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
                error_level: ErrorLevel::default(),
            }),
        _ => Err(Error::Message("failed to connect".into())),
    }
}

impl ConcatsqlConn for NonNull<ffi::sqlite3> {
    fn _execute(&self, s: &WrapString, error_level: &ErrorLevel) -> Result<()> {
        let query = s.query.as_bytes().to_vec();
        let query = match CString::new(&*query) {
            Ok(string) => string,
            _ => return Error::new(&error_level, "invalid query", &String::from_utf8(query).unwrap_or_default()),
        };
        let mut err_msg = ptr::null_mut();

        unsafe {
            ffi::sqlite3_exec(
                self.as_ptr(),
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
                unsafe{ &CStr::from_ptr(ffi::sqlite3_errmsg(self.as_ptr())).to_string_lossy().into_owned() })
        }
    }

    fn _iterate<'a>(&self, s: &WrapString, error_level: &ErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>
    {
        let query = s.query.as_bytes().to_vec();
        let query = match CString::new(&*query) {
            Ok(string) => string,
            _ => return Error::new(&error_level, "invalid query", &String::from_utf8(query).unwrap_or_default()),
        };
        let mut err_msg = ptr::null_mut();
        let callback = Box::new(callback);
        type F<'a> = &'a mut dyn FnMut(&[(&str, Option<&str>)]) -> bool;

        unsafe {
            ffi::sqlite3_exec(
                self.as_ptr(),
                query.as_ptr(),
                Some(process_callback),
                &*callback as *const F as *mut F as *mut c_void,
                &mut err_msg,
            );
        }

        if err_msg.is_null() {
            Ok(())
        } else {
            Error::new(&error_level, "exec error",
                unsafe{ &CStr::from_ptr(ffi::sqlite3_errmsg(self.as_ptr())).to_string_lossy().into_owned() })
        }
    }
}

extern "C" fn process_callback(
    callback: *mut c_void,
    count: i32,
    values: *mut *mut i8,
    columns: *mut *mut i8,
) -> i32
{
    type F<'a> = &'a mut dyn FnMut(&[(&str, Option<&str>)]) -> bool;
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
                Some(std::str::from_utf8(unsafe { CStr::from_ptr(pointer).to_bytes() }).unwrap())
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
    use concatsql::*;
    use concatsql::error::*;
    use temporary::Directory;

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
            conn.execute(prepare!("\0")),
            Err(Error::Message("invalid query".into())),
        );
        assert_eq!(
            conn.execute(prepare!("invalid query")),
            Err(Error::Message("exec error".into())),
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    fn iterate() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(
            conn.iterate(prepare!("\0"), |_| { unreachable!(); }),
            Err(Error::Message("invalid query".into())),
        );
        assert_eq!(
            conn.iterate(prepare!("invalid query"), |_| { unreachable!(); }),
            Err(Error::Message("exec error".into())),
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    fn actual_sql() {
        assert_eq!(prepare!("SELECT").actual_sql(), "SELECT");
        assert_eq!("SELECT".actual_sql(), "'SELECT'");
        assert_eq!("O'Reilly".actual_sql(), "'O''Reilly'");
        assert_eq!(prepare!("O''Reilly").actual_sql(), "O''Reilly");

        //crate::prepare!("O'Reilly").actual_sql();      // panic
        //crate::prepare!("\"O'Reilly\"").actual_sql();  // panic
    }

    #[test]
    fn debug_display() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(format!("{:?}", &conn), format!("{:?}", &conn));
    }
}

