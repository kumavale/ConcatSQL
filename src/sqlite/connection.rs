extern crate sqlite3_sys as ffi;

use std::ffi::{CStr, CString, c_void};
use std::ptr::{self, NonNull};
use std::path::Path;
use std::collections::HashSet;
use std::cell::RefCell;

use crate::Result;
use crate::OwsqlConn;
use crate::connection::{Connection, DBType};
use crate::bidimap::BidiMap;
use crate::error::{OwsqlError, OwsqlErrorLevel};
use crate::constants::OW_MINIMUM_LENGTH;
use crate::serial::SerialNumber;

/// Open a read-write connection to a new or existing database.
#[inline]
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
                conn:          Box::new(unsafe { NonNull::new_unchecked(conn_ptr) }),
                allowlist:     HashSet::new(),
                serial_number: RefCell::new(SerialNumber::default()),
                ow_len_range:  (OW_MINIMUM_LENGTH, OW_MINIMUM_LENGTH),
                overwrite:     RefCell::new(BidiMap::new()),
                error_msg:     RefCell::new(BidiMap::new()),
                error_level:   OwsqlErrorLevel::default(),
            }),
        _ => Err(OwsqlError::Message("failed to connect".into())),
    }
}

impl OwsqlConn for NonNull<ffi::sqlite3> {
    fn db_type(&self) -> DBType {
        DBType::Sqlite
    }

    fn _execute(&self, query: Result<String>, error_level: &OwsqlErrorLevel) -> Result<()> {
        let query = match query {
            Ok(query) => query,
            Err(e) => if *error_level == OwsqlErrorLevel::AlwaysOk {
                return Ok(());
            } else {
                return Err(e);
            },
        }.as_bytes().to_vec();
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

    fn _iterate<'a>(&self, query: Result<String>, error_level: &OwsqlErrorLevel,
        callback: &mut FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>
    {
        let query = match query {
            Ok(query) => query,
            Err(e) => if *error_level == OwsqlErrorLevel::AlwaysOk {
                return Ok(());
            } else {
                return Err(e);
            },
        }.as_bytes().to_vec();
        let query = match CString::new(&*query) {
            Ok(string) => string,
            _ => return OwsqlError::new(&error_level, "invalid query", &String::from_utf8(query).unwrap_or_default()),
        };
        let mut err_msg = ptr::null_mut();
        let callback = Box::new(callback);
        type F<'a> = &'a mut FnMut(&[(&str, Option<&str>)]) -> bool;

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
}

//impl Drop for SqliteConnection {
//    fn drop(&mut self) {
//        use std::thread;
//
//        let close_result = unsafe { ffi::sqlite3_close(self.raw.as_ptr()) };
//        if close_result != ffi::SQLITE_OK {
//            if thread::panicking() {
//                eprintln!("error closing SQLite connection");
//            } else {
//                panic!("error closing SQLite connection");
//            }
//        }
//    }
//}

extern "C" fn process_callback(
    callback: *mut c_void,
    count: i32,
    values: *mut *mut i8,
    columns: *mut *mut i8,
) -> i32
{
    type F<'a> = &'a mut FnMut(&[(&str, Option<&str>)]) -> bool;
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
    use crate::error::*;
    use temporary::Directory;

    #[test]
    fn open() {
        let dir = Directory::new("sqlite").unwrap();
        let path = dir.path().join("test.db");
        assert_ne!(crate::sqlite::open(""), crate::sqlite::open(""));
        assert_eq!(crate::sqlite::open(":memory:"), crate::sqlite::open(":memory:"));
        assert_eq!(crate::sqlite::open(&path), crate::sqlite::open(&path));
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
            conn.execute("\0"),
            Err(OwsqlError::Message("invalid query".into())),
        );
        assert_eq!(
            conn.execute("invalid query"),
            Err(OwsqlError::Message("exec error".into())),
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    fn iterate() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(
            conn.iterate("\0", |_| { unreachable!(); }),
            Err(OwsqlError::Message("invalid query".into())),
        );
        assert_eq!(
            conn.iterate("invalid query", |_| { unreachable!(); }),
            Err(OwsqlError::Message("exec error".into())),
        );
    }

    #[test]
    fn ow() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        //let test0: String  = String::from("test");
        //let test1: &String = &String::from("test");
        //let test2: &str    = &String::from("test");
        let test3: &'static str  = "test";
        let test4: &'static i32  = &42;
        let test5: &'static char = &'A';
        //conn.ow(test0);  // build failed
        //conn.ow(test1);  // build failed
        //conn.ow(test2);  // build failed
        conn.ow(test3);
        conn.ow(test4);
        conn.ow(test5);
        assert_eq!(conn.ow("42"), conn.ow(&42));
    }

    #[test]
    fn allowlist() {
        let mut conn = crate::sqlite::open(":memory:").unwrap();
        conn.add_allowlist(params!["Alice", "Bob", 42]);
        conn.add_allowlist(params!["O'Reilly", "\""]);
        conn.add_allowlist(params!['A', 0.123, 456.]);
        assert!(conn.is_allowlist("Alice"));
        assert!(conn.is_allowlist(&"Alice"));
        assert!(conn.is_allowlist(42));
        assert!(conn.is_allowlist(&42));
        assert!(conn.is_allowlist("O'Reilly"));
        assert!(conn.is_allowlist('"'));
        assert!(!conn.is_allowlist("Alice OR 1=1; --"));
        assert_ne!(conn.allowlist(42), conn.ow(&42));  // "'42'", "42"
        assert_ne!(conn.allowlist("Bob"), conn.ow("Bob"));  // "'Bob'", "Bob"
        assert!(conn.is_allowlist('A'));
        assert!(conn.is_allowlist("A"));
        assert!(conn.is_allowlist("0.123"));
        assert!(conn.is_allowlist("456"));
        assert!(!conn.is_allowlist("456."));
        assert!(conn.is_allowlist(0.123));
        assert!(conn.is_allowlist(456.));
        assert!(conn.is_allowlist(456.0));
        assert!(conn.is_allowlist(0.1230));
    }

    #[test]
    #[cfg(debug_assertions)]
    fn actual_sql() {
        let mut conn = crate::sqlite::open(":memory:").unwrap();
        let select = conn.ow("SELECT");
        let oreilly = conn.ow("O'Reilly");
        let allow = conn.allowlist("Alice");
        assert_eq!(conn.actual_sql(&select).unwrap(), "SELECT ");
        assert_eq!(conn.actual_sql("SELECT").unwrap(), "'SELECT' ");
        assert_eq!(conn.actual_sql(&oreilly), Err(OwsqlError::Message("invalid literal".into())));
        assert_eq!(conn.actual_sql("O'Reilly").unwrap(), "'O&#39;Reilly' ");
        assert_eq!(conn.actual_sql(&allow), Err(OwsqlError::Message("deny value".into())));
        let oreilly = conn.ow("O''Reilly");
        assert_eq!(conn.actual_sql(&oreilly), Ok("O''Reilly ".to_string()));
        let oreilly = conn.ow("\"O'Reilly\"");
        assert_eq!(conn.actual_sql(&oreilly), Ok("\"O'Reilly\" ".to_string()));
        conn.add_allowlist(params!["Alice"]);
        assert_eq!(conn.actual_sql(&allow), Err(OwsqlError::Message("deny value".into())));
        let allow = conn.allowlist("Alice");
        assert_eq!(conn.actual_sql(&allow), Ok("'Alice' ".to_string()));
    }

    #[test]
    fn debug_display() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(format!("{:?}", &conn), format!("{:?}", &conn));
    }

    #[test]
    fn set_ow_len() {
        let mut conn = crate::sqlite::open(":memory:").unwrap();
        conn.set_ow_len(0);
        conn.set_ow_len(42);
        conn.set_ow_len(0..32);
        conn.set_ow_len(0..=32);
        conn.set_ow_len(64..64);
        conn.set_ow_len(64..=64);
        conn.set_ow_len(64..32);
        conn.set_ow_len(64..=32);
    }
}

