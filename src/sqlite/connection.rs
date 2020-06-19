extern crate sqlite3_sys as ffi;

use std::ffi::{CStr, CString, c_void};
use std::ptr::{self, NonNull};
use std::path::Path;
use std::collections::HashSet;

use crate::{OwsqlError, Result};
use crate::bidimap::BidiMap;
use super::parser::check_valid_literal;
use super::Params;

use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

/// A database connection
pub struct Connection {
    raw: NonNull<ffi::sqlite3>,
    pub(crate) overwrite: BidiMap<String, String>,
    pub(crate) error_msg: BidiMap<OwsqlError, String>,
    allowlist: HashSet<String>,
}

impl Connection {
    /// Open a read-write connection to a new or existing database.
    pub fn open<T: AsRef<Path>>(path: T) -> Result<Self> {
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
            ffi::SQLITE_OPEN_CREATE | ffi::SQLITE_OPEN_READWRITE,
            ptr::null())
        };

        match open_result {
            ffi::SQLITE_OK =>
                Ok(Connection {
                    raw: unsafe { NonNull::new_unchecked(conn_ptr) },
                    overwrite: BidiMap::new(),
                    error_msg: BidiMap::new(),
                    allowlist: HashSet::new(),
                }),
            _ =>
                Err(OwsqlError::Message("failed to connect".to_string())),
        }
    }

    /// Execute a statement without processing the resulting rows if any.
    pub fn execute<T: AsRef<str>>(&self, query: T) -> Result<()> {
        let query = self.convert_to_valid_syntax(query.as_ref())?;
        let query = match CString::new(query) {
            Ok(string) => string,
            _ => return Err(OwsqlError::Message("invalid query".to_string())),
        };
        let mut err_msg = ptr::null_mut();

        unsafe {
            ffi::sqlite3_exec(
                self.raw.as_ptr(),
                query.as_ptr(),
                None,             // callback fn
                ptr::null_mut(),  // callback arg
                &mut err_msg,
            );
        }

        if err_msg.is_null() {
            Ok(())
        } else {
            Err(OwsqlError::Message("exec error".to_string()))
        }
    }

    /// Execute a statement and process the resulting rows as plain text.
    ///
    /// The callback is triggered for each row. If the callback returns `false`,
    /// no more rows will be processed.
    pub fn iterate<T: AsRef<str>, F>(&self, query: T, callback: F) -> Result<()>
        where
            F: FnMut(&[(&str, Option<&str>)]) -> bool,
    {
        let query = self.convert_to_valid_syntax(query.as_ref())?;
        let query = match CString::new(query) {
            Ok(string) => string,
            _ => return Err(OwsqlError::Message("invalid query".to_string())),
        };
        let mut err_msg = ptr::null_mut();
        let callback = Box::new(callback);

        unsafe {
            ffi::sqlite3_exec(
                self.raw.as_ptr(),
                query.as_ptr(),
                Some(process_callback::<F>),
                &*callback as *const F as *mut F as *mut _,
                &mut err_msg,
            );
        }

        if err_msg.is_null() {
            Ok(())
        } else {
            Err(OwsqlError::Message("exec error".to_string()))
        }
    }

    /// Return the overwrite definition string.
    /// All strings assembled without using this method are escaped.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut conn = owsql::sqlite::open(":memory:").unwrap();
    /// let sql = conn.ow("SELECT");
    ///
    /// assert_eq!(sql, conn.ow("SELECT"));
    /// assert_ne!(sql, "SELECT");
    /// ```
    pub fn ow<T: ?Sized + std::string::ToString>(&mut self, s: &'static T) -> String {
        let s = s.to_string();
        let result = check_valid_literal(&s);
        match result {
            Ok(_) => {
                self.overwrite.entry_or_insert(s.to_string(), overwrite_new!());
                format!(" {} ", self.overwrite.get(&s).unwrap())
            },
            Err(e) => {
                self.error_msg.entry_or_insert(e.clone(), overwrite_new!());
                format!(" {} ", self.error_msg.get(&e).unwrap())
            },
        }
    }

    /// TODO
    pub fn allowlist<T: Clone + ToString>(&mut self, value: T) -> String {
        if self.is_allowlist(value.clone()) {
            format!(" {} ", self.overwrite.get(&value.to_string()).unwrap())
        } else {
            let msg = OwsqlError::Message("deny value".to_string());
            self.error_msg.entry_or_insert(msg.clone(), overwrite_new!());
            format!(" {} ", self.error_msg.get(&msg).unwrap())
        }
    }

    ///// TODO
    //pub fn allowlist_literal<T: Clone + ToString>(&mut self, value: T) -> String {
    //    if self.is_allowlist(value.clone()) {
    //        format!(" '{}' ", self.overwrite.get(&value.to_string()).unwrap())
    //    } else {
    //        let msg = OwsqlError::Message("deny value".to_string());
    //        self.error_msg.entry_or_insert(msg.clone(), overwrite_new!());
    //        format!(" {} ", self.error_msg.get(&msg).unwrap())
    //    }
    //}

    /// TODO
    pub fn is_allowlist<T: ToString>(&mut self, value: T) -> bool {
        self.allowlist.contains(&value.to_string())
    }

    /// TODO
    pub fn add_allowlist(&mut self, params: Params) {
        params.iter().for_each(|value| {
            self.overwrite.entry_or_insert(value.to_string(), overwrite_new!());
            self.allowlist.insert(value.to_string());
        });
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        use std::thread;

        let close_result = unsafe { ffi::sqlite3_close(self.raw.as_ptr()) };
        if close_result != ffi::SQLITE_OK {
            if thread::panicking() {
                eprintln!("error closing SQLite connection");
            } else {
                panic!("error closing SQLite connection");
            }
        }
    }
}

extern "C" fn process_callback<F>(
    callback: *mut c_void,
    count: i32,
    values: *mut *mut i8,
    columns: *mut *mut i8,
) -> i32
where
    F: FnMut(&[(&str, Option<&str>)]) -> bool,
{
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
    use crate::params;

    #[test]
    fn ow() {
        let mut conn = crate::sqlite::open(":memory:").unwrap();
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
        assert!(conn.is_allowlist("Alice"));
        assert!(conn.is_allowlist(&"Alice"));
        assert!(conn.is_allowlist(42));
        assert!(conn.is_allowlist(&42));
        assert!(!conn.is_allowlist("Alice OR 1=1; --"));
        assert_eq!(conn.allowlist("Bob"), conn.ow("Bob"));
    }
}

