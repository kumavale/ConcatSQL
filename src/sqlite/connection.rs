extern crate sqlite3_sys as ffi;

use std::ffi::{CStr, CString, c_void};
use std::ptr::{self, NonNull};
use std::path::Path;
use std::collections::HashSet;
use std::fmt;
use std::cell::RefCell;

use crate::Result;
use crate::bidimap::BidiMap;
use crate::error::{OwsqlError, OwsqlErrorLevel};
use crate::constants::OW_MINIMUM_LENGTH;
use crate::overwrite::{IntoInner, overwrite_new};
use crate::serial::SerialNumber;
use super::parser::{escape_for_allowlist, single_quotaion_escape};
use super::row::Row;

/// A database connection
pub struct Connection {
    raw:                    NonNull<ffi::sqlite3>,
    allowlist:              HashSet<String>,
    serial_number:          RefCell<SerialNumber>,
    ow_len_range:           (usize, usize),
    pub(crate) overwrite:   RefCell<BidiMap<String, String>>,
    pub(crate) error_msg:   RefCell<BidiMap<OwsqlError, String>>,
    pub(crate) error_level: OwsqlErrorLevel,
}

unsafe impl Send for Connection {}
unsafe impl Sync for Connection {}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("raw", &self.raw)
            .field("error_level", &self.error_level)
            .finish()
    }
}

impl Connection {
    /// Open a read-write connection to a new or existing database.
    #[inline]
    pub fn open<T: AsRef<Path>>(path: T, openflags: i32) -> Result<Self> {
        let path = match path.as_ref().to_str() {
            Some(path) => {
                match CString::new(path) {
                    Ok(string) => string,
                    _ => return Err(OwsqlError::new(format!("invalid path: {}", path))),
                }
            },
            _ => return Err(OwsqlError::new(format!("failed to open path: {:?}", path.as_ref()))),
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
                    raw: unsafe { NonNull::new_unchecked(conn_ptr) },
                    allowlist:     HashSet::new(),
                    serial_number: RefCell::new(SerialNumber::default()),
                    ow_len_range:  (OW_MINIMUM_LENGTH, OW_MINIMUM_LENGTH),
                    overwrite:     RefCell::new(BidiMap::new()),
                    error_msg:     RefCell::new(BidiMap::new()),
                    error_level:   OwsqlErrorLevel::default(),
                }),
            _ => Err(OwsqlError::new("failed to connect")),
        }
    }

    /// Execute a statement without processing the resulting rows if any.
    ///
    /// # Examples
    ///
    /// ```
    /// # let mut conn = owsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = conn.ow(r#"CREATE TABLE users (name TEXT, id INTEGER);
    /// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
    /// # conn.execute(stmt).unwrap();
    /// let sql = conn.ow(r#"SELECT * FROM users;"#);
    /// conn.execute(&sql).unwrap();
    /// ```
    #[inline]
    pub fn execute<T: AsRef<str>>(&self, query: T) -> Result<()> {
        let query = match self.convert_to_valid_syntax(query.as_ref()) {
            Ok(query) => query,
            Err(e) => if self.error_level == OwsqlErrorLevel::AlwaysOk {
                return Ok(());
            } else {
                return Err(e);
            },
        }.as_bytes().to_vec();
        let query = match CString::new(&*query) {
            Ok(string) => string,
            _ => return self.err("invalid query", &String::from_utf8(query).unwrap_or_default()),
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
            self.err("exec error",
                unsafe{ &CStr::from_ptr(ffi::sqlite3_errmsg(self.raw.as_ptr())).to_string_lossy().into_owned() })
        }
    }

    /// Execute a statement and process the resulting rows as plain text.
    ///
    /// The callback is triggered for each row. If the callback returns `false`,
    /// no more rows will be processed.
    ///
    /// # Examples
    ///
    /// ```
    /// # let mut conn = owsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = conn.ow(r#"CREATE TABLE users (name TEXT, id INTEGER);
    /// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
    /// # conn.execute(stmt).unwrap();
    /// let sql = conn.ow(r#"SELECT * FROM users;"#);
    /// conn.iterate(&sql, |pairs| {
    ///     for &(column, value) in pairs.iter() {
    ///         println!("{} = {}", column, value.unwrap());
    ///     }
    ///     true
    /// }).unwrap();
    /// ```
    #[inline]
    pub fn iterate<T: AsRef<str>, F>(&self, query: T, callback: F) -> Result<()>
        where
            F: FnMut(&[(&str, Option<&str>)]) -> bool,
    {
        let query = match self.convert_to_valid_syntax(query.as_ref()) {
            Ok(query) => query,
            Err(e) => if self.error_level == OwsqlErrorLevel::AlwaysOk {
                return Ok(());
            } else {
                return Err(e);
            },
        }.as_bytes().to_vec();
        let query = match CString::new(&*query) {
            Ok(string) => string,
            _ => return self.err("invalid query", &String::from_utf8(query).unwrap_or_default()),
        };
        let mut err_msg = ptr::null_mut();
        let callback = Box::new(callback);

        unsafe {
            ffi::sqlite3_exec(
                self.raw.as_ptr(),
                query.as_ptr(),
                Some(process_callback::<F>),
                &*callback as *const F as *mut F as *mut c_void,
                &mut err_msg,
            );
        }

        if err_msg.is_null() {
            Ok(())
        } else {
            self.err("exec error",
                unsafe{ &CStr::from_ptr(ffi::sqlite3_errmsg(self.raw.as_ptr())).to_string_lossy().into_owned() })
        }
    }

    /// Execute a statement and returns the rows.
    ///
    /// # Examples
    ///
    /// ```
    /// # let mut conn = owsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = conn.ow(r#"CREATE TABLE users (name TEXT, id INTEGER);
    /// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
    /// # conn.execute(stmt).unwrap();
    /// let sql = conn.ow(r#"SELECT name FROM users;"#);
    /// let rows = conn.rows(&sql).unwrap();
    /// for row in rows.iter() {
    ///     println!("name: {}", row.get("name").unwrap_or("NULL"));
    /// }
    /// ```
    #[inline]
    pub fn rows<T: AsRef<str>>(&self, query: T) -> Result<Vec<Row>> {
        let mut rows: Vec<Row> = Vec::new();

        self.iterate(query, |pairs| {
            let mut row = Row::new();
            for &(column, value) in pairs.iter() {
                row.insert(column.to_string(), value.map(|v| v.to_string()));
            }
            rows.push(row);
            true
        })?;

        Ok(rows)
    }

    /// Return the actual SQL statement.
    ///
    /// # Examples
    ///
    /// ```
    /// # use owsql::error::OwsqlError;
    /// let mut conn = owsql::sqlite::open(":memory:").unwrap();
    /// let select = conn.ow("SELECT");
    /// let oreilly = conn.ow("O'Reilly");
    /// let oreilly_unhtmlescape = unsafe { conn.ow_without_html_escape("O'Reilly") };
    /// assert_eq!(conn.actual_sql(&select).unwrap(), "SELECT ");
    /// assert_eq!(conn.actual_sql("SELECT").unwrap(), "'SELECT' ");
    /// assert_eq!(conn.actual_sql(&oreilly), Err(OwsqlError::Message("invalid literal".to_string())));
    /// assert_eq!(conn.actual_sql("O'Reilly").unwrap(), "'O&#39;Reilly' ");
    /// assert_eq!(conn.actual_sql(&oreilly_unhtmlescape).unwrap(), "'O''Reilly' ");
    /// ```
    #[inline]
    pub fn actual_sql<T: AsRef<str>>(&self, query: T) -> Result<String> {
        self.convert_to_valid_syntax(query.as_ref())
    }

    /// Return the overwrite definition string.  
    /// All strings assembled without using this method are escaped.  
    /// This method does not sanitize.  
    /// A string containing incomplete quotes like the one below will result in an error.  
    ///
    /// # Errors
    ///
    /// ```rust,ignore
    /// conn.ow("where name = 'foo' OR name = '") + name + &conn.ow("';");
    ///                                       ^                      ^
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// # let mut conn = owsql::sqlite::open(":memory:").unwrap();
    /// let sql = conn.ow("SELECT");
    ///
    /// assert_eq!(sql, conn.ow("SELECT"));
    /// assert_ne!(sql, "SELECT");
    /// ```
    #[inline]
    pub fn ow<T: ?Sized + std::string::ToString>(&self, s: &'static T) -> String {
        let s = s.to_string();
        let result = self.check_valid_literal(&s);
        match result {
            Ok(_) => {
                let s = s.to_string();
                if !self.overwrite.borrow_mut().contain(&s) {
                    let overwrite = overwrite_new(self.serial_number.borrow_mut().get(), self.ow_len_range);
                    self.overwrite.borrow_mut().insert(s.to_string(), overwrite);
                }
                format!(" {} ", self.overwrite.borrow_mut().get(&s).unwrap())
            },
            Err(e) => {
                if !self.error_msg.borrow_mut().contain(&e) {
                    let overwrite = overwrite_new(self.serial_number.borrow_mut().get(), self.ow_len_range);
                    self.error_msg.borrow_mut().insert(e.clone(), overwrite);
                }
                format!(" {} ", self.error_msg.borrow_mut().get(&e).unwrap())
            },
        }
    }

    /// Return the overwrite definition string without HTML escape.  
    ///
    /// # Safety
    ///
    /// This is an unsafe method!! => I am considering whether to use the unsafe keyword :(  
    /// Note that this can be XSS.
    #[inline]
    pub unsafe fn ow_without_html_escape<T: Clone + ToString>(&self, value: T) -> String {
        let s = format!("'{}'", single_quotaion_escape(&value.to_string()));
        let result = self.check_valid_literal(&s);
        match result {
            Ok(_) => {
                let s = s.to_string();
                if !self.overwrite.borrow_mut().contain(&s) {
                    let overwrite = overwrite_new(self.serial_number.borrow_mut().get(), self.ow_len_range);
                    self.overwrite.borrow_mut().insert(s.to_string(), overwrite);
                }
                format!(" {} ", self.overwrite.borrow_mut().get(&s).unwrap())
            },
            Err(e) => {
                if !self.error_msg.borrow_mut().contain(&e) {
                    let overwrite = overwrite_new(self.serial_number.borrow_mut().get(), self.ow_len_range);
                    self.error_msg.borrow_mut().insert(e.clone(), overwrite);
                }
                format!(" {} ", self.error_msg.borrow_mut().get(&e).unwrap())
            },
        }
    }

    /// Return the overwrite definition string in allowlist.  
    /// Returns the escaped string.  
    ///
    /// # Examples
    ///
    /// ```
    /// # use owsql::params;
    /// # let mut conn = owsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = conn.ow(r#"CREATE TABLE users (name TEXT, id INTEGER);
    /// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
    /// # conn.execute(stmt).unwrap();
    /// conn.add_allowlist(params!["Alice", "Bob"]);
    /// let input = "Alice OR 1=1; --";
    /// let sql = conn.ow("SELECT * FROM users WHERE name = ") + &conn.allowlist(input);
    ///
    /// assert!(conn.execute(sql).is_err());
    /// ```
    #[inline]
    pub fn allowlist<T: Clone + ToString>(&self, value: T) -> String {
        if self.is_allowlist(value.clone()) {
            format!(" {} ", self.overwrite.borrow_mut().get(&escape_for_allowlist(&value.to_string())).unwrap())
        } else {
            let e = self.err("deny value", &value.to_string()).err().unwrap_or(OwsqlError::AnyError);
            if !self.error_msg.borrow_mut().contain(&e) {
                let overwrite = overwrite_new(self.serial_number.borrow_mut().get(), self.ow_len_range);
                self.error_msg.borrow_mut().insert(e.clone(), overwrite);
            }
            format!(" {} ", self.error_msg.borrow_mut().get(&e).unwrap())
        }
    }

    /// Checks if the value is within the allowlist.
    ///
    /// # Examples
    ///
    /// ```
    /// # use owsql::params;
    /// # let mut conn = owsql::sqlite::open(":memory:").unwrap();
    /// conn.add_allowlist(params!["Alice", "Bob", 42, 123]);
    /// assert!(conn.is_allowlist("Alice"));
    /// assert!(!conn.is_allowlist("'Alice'"));
    /// assert!(conn.is_allowlist(42));
    /// assert!(conn.is_allowlist("42"));
    /// assert!(!conn.is_allowlist("'42'"));
    /// ```
    #[inline]
    pub fn is_allowlist<T: ToString>(&self, value: T) -> bool {
        self.allowlist.contains(&value.to_string())
    }

    /// Register it in self.overwrite after performing character string escape processing with
    /// single quotation added to both sides.  
    /// Use [params macro](../macro.params.html).  
    ///
    /// # Examples
    ///
    /// ```
    /// # use owsql::params;
    /// # let mut conn = owsql::sqlite::open(":memory:").unwrap();
    /// conn.add_allowlist(params!["Alice", 'A', 42, 0.123]);
    /// ```
    #[inline]
    pub fn add_allowlist(&mut self, params: Vec<crate::value::Value>) {
        for value in params {
            self.allowlist.insert(value.to_string());
            self.overwrite.borrow_mut().insert(
                escape_for_allowlist(&value.to_string()),
                overwrite_new(self.serial_number.borrow_mut().get(), self.ow_len_range)
            );
        }
    }

    /// It is guaranteed to be a signed 64-bit integer without quotation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use owsql::params;
    /// # let mut conn = owsql::sqlite::open(":memory:").unwrap();
    /// conn.int(42);              // ok
    /// conn.int("42");            // ok
    /// conn.int("42 or 1=1; --"); // error
    /// ```
    #[inline]
    pub fn int<T: Clone + ToString>(&self, value: T) -> String {
        let value = value.to_string();
        if value.parse::<i64>().is_ok() {
            let value = value.to_string();
            if !self.overwrite.borrow_mut().contain(&value) {
                let overwrite = overwrite_new(self.serial_number.borrow_mut().get(), self.ow_len_range);
                self.overwrite.borrow_mut().insert(value.to_string(), overwrite);
            }
            format!(" {} ", self.overwrite.borrow_mut().get(&value).unwrap())
        } else {
            let e = self.err("non integer", &value).err().unwrap_or(OwsqlError::AnyError);
            if !self.error_msg.borrow_mut().contain(&e) {
                let overwrite = overwrite_new(self.serial_number.borrow_mut().get(), self.ow_len_range);
                self.error_msg.borrow_mut().insert(e.clone(), overwrite);
            }
            format!(" {} ", self.error_msg.borrow_mut().get(&e).unwrap())
        }
    }

    /// You can set a different fixed value or a different length each time.  
    /// The [ow method](./struct.Connection.html#method.ow) outputs a random number of about 32
    /// digits by default.  
    /// However, if a number less than 32 digits is entered, it will be set to 32 digits.  
    ///
    /// # Examples
    ///
    /// ```
    /// # use owsql::params;
    /// # let mut conn = owsql::sqlite::open(":memory:").unwrap();
    /// conn.set_ow_len(42);       // 42
    /// conn.set_ow_len(50..100);  // 50-99
    /// conn.set_ow_len(50..=100); // 50-100
    /// ```
    #[inline]
    pub fn set_ow_len<T: 'static + IntoInner>(&mut self, range: T) {
        self.ow_len_range = {
            let range = range.into_inner();
            let range0 = if range.0 < OW_MINIMUM_LENGTH { OW_MINIMUM_LENGTH } else { range.0 };
            let range1 = if range.1 < OW_MINIMUM_LENGTH { OW_MINIMUM_LENGTH } else { range.1 };
            (range0, range1)
        };
    }

    /// Sets the error level.  
    /// Default is [OwsqlErrorLevel](../error/enum.OwsqlErrorLevel.html)::Release.  
    /// Values can be changed only during debug build.
    ///
    /// # Examples
    ///
    /// ```
    /// # use owsql::error::OwsqlErrorLevel;
    /// # let mut conn = owsql::sqlite::open(":memory:").unwrap();
    /// conn.error_level(OwsqlErrorLevel::Develop);
    /// ```
    #[inline]
    pub fn error_level(&mut self, level: OwsqlErrorLevel) {
        // Values can be changed only during debug build
        if cfg!(debug_assertions) {
            self.error_level = level;
        }
    }

    #[inline]
    pub(crate) fn err(&self, err_msg: &str, detail_msg: &str) -> Result<(), OwsqlError> {
        match self.error_level {
            OwsqlErrorLevel::AlwaysOk => Ok(()),
            OwsqlErrorLevel::Release  => Err(OwsqlError::AnyError),
            OwsqlErrorLevel::Develop  => Err(OwsqlError::new(&err_msg)),
            OwsqlErrorLevel::Debug    => Err(OwsqlError::new(&format!("{}: {}", err_msg, detail_msg))),
        }
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
    use crate::*;
    use crate::error::*;

    #[test]
    fn open() {
        assert_ne!(crate::sqlite::open(""), crate::sqlite::open(""));
        assert_ne!(crate::sqlite::open(":memory:"), crate::sqlite::open(":memory:"));
        #[cfg(unix)]
        assert_ne!(crate::sqlite::open("/tmp/tmp.db"), crate::sqlite::open("/tmp/tmp.db"));
        assert_eq!(
            crate::sqlite::open("foo\0bar"),
            Err(OwsqlError::new("invalid path: foo\u{0}bar"))
        );
    }

    #[test]
    fn execute() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(
            conn.execute("\0"),
            Err(OwsqlError::new("invalid query")),
        );
        assert_eq!(
            conn.execute("invalid query"),
            Err(OwsqlError::new("exec error")),
        );
    }

    #[test]
    fn iterate() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(
            conn.iterate("\0", |_| { unreachable!(); }),
            Err(OwsqlError::new("invalid query")),
        );
        assert_eq!(
            conn.iterate("invalid query", |_| { unreachable!(); }),
            Err(OwsqlError::new("exec error")),
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
    fn actual_sql() {
        let mut conn = crate::sqlite::open(":memory:").unwrap();
        let select = conn.ow("SELECT");
        let oreilly = conn.ow("O'Reilly");
        let allow = conn.allowlist("Alice");
        assert_eq!(conn.actual_sql(&select).unwrap(), "SELECT ");
        assert_eq!(conn.actual_sql("SELECT").unwrap(), "'SELECT' ");
        assert_eq!(conn.actual_sql(&oreilly), Err(OwsqlError::new("invalid literal")));
        assert_eq!(conn.actual_sql("O'Reilly").unwrap(), "'O&#39;Reilly' ");
        assert_eq!(conn.actual_sql(&allow), Err(OwsqlError::new("deny value")));
        let oreilly = conn.ow("O''Reilly");
        assert_eq!(conn.actual_sql(&oreilly), Ok("O''Reilly ".to_string()));
        let oreilly = conn.ow("\"O'Reilly\"");
        assert_eq!(conn.actual_sql(&oreilly), Ok("\"O'Reilly\" ".to_string()));
        conn.add_allowlist(params!["Alice"]);
        assert_eq!(conn.actual_sql(&allow), Err(OwsqlError::new("deny value")));
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

