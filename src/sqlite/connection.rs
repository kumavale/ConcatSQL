extern crate sqlite3_sys as ffi;

use std::ffi::{CStr, CString, c_void};
use std::ptr::{self, NonNull};
use std::path::Path;

use crate::Result;
use crate::connection::{Connection, OwsqlConn};
use crate::error::{OwsqlError, OwsqlErrorLevel};
use crate::owstring::OwString;

/// Open a read-write connection to a new or existing database.
pub fn open<T: AsRef<Path>>(path: T, openflags: i32) -> Result<Connection> {
    let path = match path.as_ref().to_str() {
        Some(path) => {
            match CString::new(path) {
                Ok(string) => string,
                _ => return Err(OwsqlError::Message(format!("invalid path: {}", path))),
            }
        },
        _ => return Err(OwsqlError::Message(format!("failed to open path: {:?}", path.as_ref()))),
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
                conn:              Box::new(unsafe { NonNull::new_unchecked(conn_ptr) }),
                error_level:       OwsqlErrorLevel::default(),
            }),
        _ => Err(OwsqlError::Message("failed to connect".into())),
    }
}

impl OwsqlConn for NonNull<ffi::sqlite3> {
    fn _execute(&self, s: &OwString, error_level: &OwsqlErrorLevel) -> Result<()> {
        let query = s.query.as_bytes().to_vec();
        let query = match CString::new(&*query) {
            Ok(string) => string,
            _ => return OwsqlError::new(&error_level, "invalid query", &String::from_utf8(query).unwrap_or_default()),
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
            OwsqlError::new(&error_level, "exec error",
                unsafe{ &CStr::from_ptr(ffi::sqlite3_errmsg(self.as_ptr())).to_string_lossy().into_owned() })
        }
    }

    fn _iterate<'a>(&self, s: &OwString, error_level: &OwsqlErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>
    {
        let query = s.query.as_bytes().to_vec();
        let query = match CString::new(&*query) {
            Ok(string) => string,
            _ => return OwsqlError::new(&error_level, "invalid query", &String::from_utf8(query).unwrap_or_default()),
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
            OwsqlError::new(&error_level, "exec error",
                unsafe{ &CStr::from_ptr(ffi::sqlite3_errmsg(self.as_ptr())).to_string_lossy().into_owned() })
        }
    }

    fn must_escape(&self) -> Box<dyn Fn(char) -> bool> {
        Box::new(|c| c == '\'')
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
    use crate::*;
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
            Err(OwsqlError::Message("invalid path: foo\u{0}bar".into()))
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    fn execute() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(
            conn.execute(conn.prepare("\0")),
            Err(OwsqlError::Message("invalid query".into())),
        );
        assert_eq!(
            conn.execute(conn.prepare("invalid query")),
            Err(OwsqlError::Message("exec error".into())),
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    fn iterate() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(
            conn.iterate(conn.prepare("\0"), |_| { unreachable!(); }),
            Err(OwsqlError::Message("invalid query".into())),
        );
        assert_eq!(
            conn.iterate(conn.prepare("invalid query"), |_| { unreachable!(); }),
            Err(OwsqlError::Message("exec error".into())),
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    fn actual_sql() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(conn.prepare("SELECT").actual_sql(), "SELECT");
        assert_eq!(conn.bind("SELECT").actual_sql(), "'SELECT'");
        //assert_eq!(conn.prepare("O'Reilly").actual_sql(), "O''Reilly"); // panic
        assert_eq!(conn.bind("O'Reilly").actual_sql(), "'O''Reilly'");
        assert_eq!(conn.prepare("O''Reilly").actual_sql(), "O''Reilly");
        //assert_eq!(conn.prepare("\"O'Reilly\"").actual_sql(), "\"O'Reilly\""); // panic
    }

    #[test]
    fn debug_display() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(format!("{:?}", &conn), format!("{:?}", &conn));
    }
}

