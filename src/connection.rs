use std::fmt;
use std::cell::Cell;
use std::borrow::Cow;

use crate::Result;
use crate::ErrorLevel;
use crate::row::Row;
use crate::wrapstring::{WrapString, IntoWrapString};
use crate::value::Value;

pub(crate) trait ConcatsqlConn {
    fn execute_inner<'a>(&self, query: Cow<'a, str>, params: &[Value<'a>], error_level: &crate::ErrorLevel) -> Result<()>;
    fn iterate_inner<'a>(&self, query: Cow<'a, str>, params: &[Value<'a>], error_level: &crate::ErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>;
    fn rows_inner<'a, 'r>(&self, query: Cow<'a, str>, params: &[Value<'a>], error_level: &crate::ErrorLevel)
        -> Result<Vec<Row<'r>>>;
    fn close(&self);
    fn kind(&self) -> ConnKind;
}

#[doc(hidden)]
pub enum ConnKind {
    #[cfg(feature = "sqlite")]   SQLite,
    #[cfg(feature = "mysql")]    MySQL,
    #[cfg(feature = "postgres")] PostgreSQL,
}

/// A database connection.
pub struct Connection {
    pub(crate) conn:        Box<dyn ConcatsqlConn>,
    pub(crate) error_level: Cell<ErrorLevel>,
}

unsafe impl Send for Connection {}
unsafe impl Sync for Connection {}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.conn, &other.conn)
    }
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("conn", &(&self.conn as *const _))
            .field("error_level", &self.error_level.get())
            .finish()
    }
}

impl<'a> Connection {
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
    pub fn execute<T: IntoWrapString<'a>>(&self, query: T) -> Result<()> {
        self.conn.execute_inner(query.compile(self.conn.kind()), query.params(), &self.error_level.get())
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
    pub fn iterate<T: IntoWrapString<'a>, F>(&self, query: T, mut callback: F) -> Result<()>
        where
            F: FnMut(&[(&str, Option<&str>)]) -> bool,
    {
        self.conn.iterate_inner(query.compile(self.conn.kind()), query.params(), &self.error_level.get(), &mut callback)
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
    #[inline]
    pub fn rows<'r, T: IntoWrapString<'a>>(&self, query: T) -> Result<Vec<Row<'r>>> {
        self.conn.rows_inner(query.compile(self.conn.kind()), query.params(), &self.error_level.get())
    }

    /// Sets the error level.  
    /// The default value is [ErrorLevel](./enum.ErrorLevel.html)::Develop for debug builds and [ErrorLevel](./enum.ErrorLevel.html)::Release for release builds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::ErrorLevel;
    /// # let conn = concatsql::sqlite::open(":memory:").unwrap();
    /// conn.error_level(ErrorLevel::AlwaysOk);
    /// ```
    pub fn error_level(&self, level: ErrorLevel) {
        self.error_level.set(level);
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.conn.close();
    }
}

/// Does not escape.
///
/// Don't use if the value entered is unreliable (e.g. entered by user).  
///
/// # Danger
///
/// ```
/// # use concatsql::prelude::*;
/// # let conn = concatsql::sqlite::open(":memory:").unwrap();
/// # let stmt = r#"CREATE TABLE users (name TEXT, age INTEGER);
/// #               INSERT INTO users (name, age) VALUES ('Alice', 42);
/// #               INSERT INTO users (name, age) VALUES ('Bob',   69);"#;
/// # conn.execute(stmt).unwrap();
/// let age = String::from("42 or 1=1; --");  // input by attcker
/// let sql = prep!("SELECT name FROM users WHERE age < ") + unsafe { without_escape(&age) };
/// assert_eq!(sql.simulate(), "SELECT name FROM users WHERE age < 42 or 1=1; --");
/// assert!(conn.rows(&sql).is_ok());
/// ```
///
/// # Safety
///
/// - Use trusted values
/// - Use in an environment where SQL injection does not occur
pub unsafe fn without_escape<T: ?Sized + ToString>(query: &T) -> WrapString {
    WrapString::new(query)
}

