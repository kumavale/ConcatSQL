use std::fmt;

use crate::Result;
use crate::ConcatsqlErrorLevel;
use crate::row::Row;
use crate::wrapstring::WrapString;

pub(crate) trait ConcatsqlConn {
    fn _execute(&self, query: &WrapString, error_level: &crate::ConcatsqlErrorLevel) -> Result<()>;
    fn _iterate(&self, query: &WrapString, error_level: &crate::ConcatsqlErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>;
}

/// A database connection.
pub struct Connection {
    pub(crate) conn:        Box<dyn ConcatsqlConn>,
    pub(crate) error_level: ConcatsqlErrorLevel,
}

unsafe impl Send for Connection {}
unsafe impl Sync for Connection {}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        (&self.conn as *const _) == (&other.conn as *const _)
    }
}

impl fmt::Debug for Connection {
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

impl Connection {
    /// Execute a statement without processing the resulting rows if any.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::{prepare, bind};
    /// # let conn = concatsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = prepare!(r#"CREATE TABLE users (name TEXT, id INTEGER);
    /// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
    /// # conn.execute(stmt).unwrap();
    /// let sql = prepare!("SELECT * FROM users;");
    /// conn.execute(&sql).unwrap();
    /// ```
    #[inline]
    pub fn execute<T: AsRef<WrapString>>(&self, query: T) -> Result<()> {
        self.conn._execute(query.as_ref(), &self.error_level)
    }

    /// Execute a statement and process the resulting rows as plain text.
    ///
    /// The callback is triggered for each row. If the callback returns `false`,
    /// no more rows will be processed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::{prepare, bind};
    /// # let conn = concatsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = prepare!(r#"CREATE TABLE users (name TEXT, id INTEGER);
    /// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
    /// # conn.execute(stmt).unwrap();
    /// let sql = prepare!("SELECT * FROM users;");
    /// conn.iterate(&sql, |pairs| {
    ///     for &(column, value) in pairs.iter() {
    ///         println!("{} = {}", column, value.unwrap());
    ///     }
    ///     true
    /// }).unwrap();
    /// ```
    #[inline]
    pub fn iterate<T: AsRef<WrapString>, F>(&self, query: T, mut callback: F) -> Result<()>
        where
            F: FnMut(&[(&str, Option<&str>)]) -> bool,
    {
        self.conn._iterate(query.as_ref(), &self.error_level, &mut callback)
    }

    /// Execute a statement and returns the rows.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::{prepare, bind};
    /// # let conn = concatsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = prepare!(r#"CREATE TABLE users (name TEXT, id INTEGER);
    /// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
    /// # conn.execute(stmt).unwrap();
    /// let sql = prepare!("SELECT name FROM users;");
    /// let rows = conn.rows(&sql).unwrap();
    /// for row in rows.iter() {
    ///     println!("name: {}", row.get("name").unwrap_or("NULL"));
    /// }
    /// ```
    pub fn rows<T: AsRef<WrapString>>(&self, query: T) -> Result<Vec<Row>> {
        let mut rows: Vec<Row> = Vec::new();

        self.iterate(query, |pairs| {
            let mut row = Row::new();
            for (column, value) in pairs.iter() {
                row.insert((*column).to_string(), value.map(|v| v.to_string()));
            }
            rows.push(row);
            true
        })?;

        Ok(rows)
    }

    /// Does not escape.  
    /// Don't use if the value entered is unreliable (e.g. entered by user).  
    ///
    /// # Danger
    ///
    /// ```
    /// # use concatsql::{prepare, bind};
    /// # let conn = concatsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = prepare!(r#"CREATE TABLE users (name TEXT, age INTEGER);
    /// #               INSERT INTO users (name, age) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, age) VALUES ('Bob',   69);"#);
    /// # conn.execute(stmt).unwrap();
    /// let age = String::from("42 or 1=1; --");  // input by attcker
    /// let sql = prepare!("SELECT name FROM users WHERE age < ") + unsafe { conn.without_escape(&age) };
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

    /// It is guaranteed to be a signed 64-bit integer without quotation.
    ///
    /// # Examples
    ///
    /// ```
    /// # let conn = concatsql::sqlite::open(":memory:").unwrap();
    /// conn.int(42);              // ok
    /// conn.int("42");            // ok
    /// conn.int("42 or 1=1; --"); // error
    /// ```
    pub fn int<T: Clone + ToString>(&self, value: T) -> Result<WrapString, &str> {
        let value = value.to_string();
        if value.parse::<i64>().is_ok() {
            Ok(WrapString::new(&value))
        } else {
            Err("not integer")
        }
    }

    /// Sets the error level.  
    /// The default value is [ConcatsqlErrorLevel](./enum.ConcatsqlErrorLevel.html)::Develop for debug builds and [ConcatsqlErrorLevel](./enum.ConcatsqlErrorLevel.html)::Release for release builds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use concatsql::ConcatsqlErrorLevel;
    /// # let mut conn = concatsql::sqlite::open(":memory:").unwrap();
    /// conn.error_level(ConcatsqlErrorLevel::Debug);
    /// ```
    pub fn error_level(&mut self, level: ConcatsqlErrorLevel) {
        self.error_level = level;
    }
}

