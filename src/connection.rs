use std::fmt;
use std::pin::Pin;

use crate::Result;
use crate::ErrorLevel;
use crate::row::Row;
use crate::wrapstring::WrapString;

pub(crate) trait ConcatsqlConn {
    fn execute_inner(&self, query: &str, error_level: &crate::ErrorLevel) -> Result<()>;
    fn iterate_inner(&self, query: &str, error_level: &crate::ErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>;
    fn rows_inner(&self, query: &str, error_level: &crate::ErrorLevel) -> Result<Vec<Row>>;
    fn kind(&self) -> ConnKind;
}

pub(crate) enum ConnKind {
    #[cfg(feature = "sqlite")]   SQLite,
    #[cfg(feature = "mysql")]    MySQL,
    #[cfg(feature = "postgres")] PostgreSQL,
}

/// A database connection.
pub struct Connection<'a> {
    pub(crate) conn:        Pin<&'a dyn ConcatsqlConn>,
    pub(crate) error_level: ErrorLevel,
}

unsafe impl<'a> Send for Connection<'a> {}
unsafe impl<'a> Sync for Connection<'a> {}

impl<'a> PartialEq for Connection<'a> {
    fn eq(&self, other: &Self) -> bool {
        (&self.conn as *const _) == (&other.conn as *const _)
    }
}

impl<'a> fmt::Debug for Connection<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("conn", &(&self.conn as *const _))
            .field("error_level", &self.error_level)
            .finish()
    }
}

impl AsRef<WrapString> for WrapString {
    #[inline]
    fn as_ref(&self) -> &WrapString {
        self
    }
}

/// Hard-coded string or escaped string.
///
/// - &'static [str](https://doc.rust-lang.org/std/primitive.str.html)
/// - [WrapString](./struct.WrapString.html)
/// - &[WrapString](./struct.WrapString.html)
pub trait SafeStr {
    /// Extracts a string slice containing the entire [WrapString](./struct.WrapString.html).
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::prelude::*;
    /// let s = prep!("foo");
    ///
    /// assert_eq!("foo", s.as_str());
    /// ```
    fn as_str(&self) -> &str;
}
impl SafeStr for &'static str {
    #[doc(hidden)]
    fn as_str(&self) -> &str {
        self
    }
}
impl SafeStr for WrapString {
    #[doc(hidden)]
    fn as_str(&self) -> &str {
        &self.query
    }
}
impl SafeStr for &WrapString {
    #[doc(hidden)]
    fn as_str(&self) -> &str {
        &self.query
    }
}

impl<'a> Connection<'a> {
    /// Execute a statement without processing the resulting rows if any.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::prep;
    /// # let conn = concatsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = r#"CREATE TABLE users (name TEXT, id INTEGER);
    /// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#;
    /// # conn.execute(stmt).unwrap();
    /// conn.execute("SELECT * FROM users;").unwrap();
    /// conn.execute(prep!("SELECT * FROM users;")).unwrap();
    /// ```
    #[inline]
    pub fn execute<T: SafeStr>(&self, query: T) -> Result<()> {
        self.conn.execute_inner(query.as_str(), &self.error_level)
    }

    /// Execute a statement and process the resulting rows as plain text.
    ///
    /// The callback is triggered for each row. If the callback returns `false`,
    /// no more rows will be processed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::prep;
    /// # let conn = concatsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = r#"CREATE TABLE users (name TEXT, id INTEGER);
    /// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#;
    /// # conn.execute(stmt).unwrap();
    /// let sql = prep!("SELECT * FROM users;");
    /// conn.iterate(&sql, |pairs| {
    ///     for &(column, value) in pairs.iter() {
    ///         println!("{} = {}", column, value.unwrap());
    ///     }
    ///     true
    /// }).unwrap();
    /// ```
    #[inline]
    pub fn iterate<T: SafeStr, F>(&self, query: T, mut callback: F) -> Result<()>
        where
            F: FnMut(&[(&str, Option<&str>)]) -> bool,
    {
        self.conn.iterate_inner(query.as_str(), &self.error_level, &mut callback)
    }

    /// Execute a statement and returns the rows.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::prep;
    /// # let conn = concatsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = r#"CREATE TABLE users (name TEXT, id INTEGER);
    /// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#;
    /// # conn.execute(stmt).unwrap();
    /// let sql = prep!("SELECT name FROM users;");
    /// let rows = conn.rows(&sql).unwrap();
    /// for row in rows {
    ///     println!("name: {}", row.get("name").unwrap_or("NULL"));
    /// }
    /// ```
    pub fn rows<T: SafeStr>(&self, query: T) -> Result<Vec<Row>> {
        self.conn.rows_inner(query.as_str(), &self.error_level)
    }

    /// Does not escape.  
    /// Don't use if the value entered is unreliable (e.g. entered by user).  
    ///
    /// # Danger
    ///
    /// ```
    /// # use concatsql::{prep, Wrap};
    /// # let conn = concatsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = r#"CREATE TABLE users (name TEXT, age INTEGER);
    /// #               INSERT INTO users (name, age) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, age) VALUES ('Bob',   69);"#;
    /// # conn.execute(stmt).unwrap();
    /// let age = String::from("42 or 1=1; --");  // input by attcker
    /// let sql = prep!("SELECT name FROM users WHERE age < ") + unsafe { conn.without_escape(&age) };
    /// assert_eq!(sql.actual_sql(), "SELECT name FROM users WHERE age < 42 or 1=1; --");
    /// assert!(conn.rows(&sql).is_ok());
    /// ```
    ///
    /// # Safety
    ///
    /// - Use trusted values
    /// - Use in an environment where SQL injection does not occur
    pub unsafe fn without_escape<T: ?Sized + ToString>(&self, query: &T) -> WrapString {
        WrapString::new(query)
    }

    /// Sets the error level.  
    /// The default value is [ErrorLevel](./enum.ErrorLevel.html)::Develop for debug builds and [ErrorLevel](./enum.ErrorLevel.html)::Release for release builds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::ErrorLevel;
    /// # let mut conn = concatsql::sqlite::open(":memory:").unwrap();
    /// conn.error_level(ErrorLevel::Debug);
    /// ```
    pub fn error_level(&mut self, level: ErrorLevel) {
        self.error_level = level;
    }
}

impl<'a> Drop for Connection<'a> {
    fn drop(&mut self) {
        match self.conn.kind() {
            #[cfg(feature = "sqlite")]
            ConnKind::SQLite => {
                unsafe {
                    extern crate sqlite3_sys as ffi;
                    ffi::sqlite3_busy_handler(&*self.conn as *const _ as *mut ffi::sqlite3, None, std::ptr::null_mut());
                    let close_result = ffi::sqlite3_close(&*self.conn as *const _ as *mut ffi::sqlite3);
                    std::ptr::drop_in_place(&*self.conn as *const _ as *mut ffi::sqlite3);
                    if close_result != ffi::SQLITE_OK {
                        eprintln!("error closing SQLite connection: {}", close_result);
                    }
                }
            }
            #[cfg(feature = "mysql")]
            ConnKind::MySQL => {}
            #[cfg(feature = "postgres")]
            ConnKind::PostgreSQL => {}
        }
    }
}

