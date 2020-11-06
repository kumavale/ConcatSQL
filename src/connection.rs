use std::fmt;

use crate::Result;
use crate::{OwsqlError, OwsqlErrorLevel};
use crate::parser::*;
use crate::row::Row;
use crate::owstring::OwString;

pub(crate) trait OwsqlConn {
    fn _execute(&self, query: &OwString, error_level: &crate::OwsqlErrorLevel) -> Result<()>;
    fn _iterate(&self, query: &OwString, error_level: &crate::OwsqlErrorLevel,
        callback: &mut dyn FnMut(&[(&str, Option<&str>)]) -> bool) -> Result<()>;
    fn must_escape(&self) -> Box<dyn Fn(char) -> bool>;
}

/// A database connection.
pub struct Connection {
    pub(crate) conn:        Box<dyn OwsqlConn>,
    pub(crate) error_level: OwsqlErrorLevel,
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

impl AsRef<OwString> for OwString {
    #[inline]
    fn as_ref(&self) -> &OwString {
        self
    }
}

impl Connection {
    /// Execute a statement without processing the resulting rows if any.
    ///
    /// # Examples
    ///
    /// ```
    /// # let conn = exowsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = conn.prepare(r#"CREATE TABLE users (name TEXT, id INTEGER);
    /// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
    /// # conn.execute(stmt).unwrap();
    /// let sql = conn.prepare("SELECT * FROM users;");
    /// conn.execute(&sql).unwrap();
    /// ```
    #[inline]
    pub fn execute<T: AsRef<OwString>>(&self, query: T) -> Result<()> {
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
    /// # let conn = exowsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = conn.prepare(r#"CREATE TABLE users (name TEXT, id INTEGER);
    /// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
    /// # conn.execute(stmt).unwrap();
    /// let sql = conn.prepare("SELECT * FROM users;");
    /// conn.iterate(&sql, |pairs| {
    ///     for &(column, value) in pairs.iter() {
    ///         println!("{} = {}", column, value.unwrap());
    ///     }
    ///     true
    /// }).unwrap();
    /// ```
    #[inline]
    pub fn iterate<T: AsRef<OwString>, F>(&self, query: T, mut callback: F) -> Result<()>
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
    /// # let conn = exowsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = conn.prepare(r#"CREATE TABLE users (name TEXT, id INTEGER);
    /// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
    /// # conn.execute(stmt).unwrap();
    /// let sql = conn.prepare("SELECT name FROM users;");
    /// let rows = conn.rows(&sql).unwrap();
    /// for row in rows.iter() {
    ///     println!("name: {}", row.get("name").unwrap_or("NULL"));
    /// }
    /// ```
    pub fn rows<T: AsRef<OwString>>(&self, query: T) -> Result<Vec<Row>> {
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

    pub fn prepare(&self, query: &'static str) -> OwString {
        check_valid_literal(query, &self.error_level).unwrap();
        OwString::new(query)
    }

    pub fn bind<T: ToString>(&self, value: T) -> OwString {
        let escaped = escape_string(&value.to_string(), self.conn.must_escape());
        OwString::new(&escaped)
    }

    /// Does not escape.  
    /// Don't use if the value entered is unreliable (e.g. entered by user).  
    ///
    /// # Danger
    ///
    /// ```
    /// # let conn = exowsql::sqlite::open(":memory:").unwrap();
    /// # let stmt = conn.prepare(r#"CREATE TABLE users (name TEXT, age INTEGER);
    /// #               INSERT INTO users (name, age) VALUES ('Alice', 42);
    /// #               INSERT INTO users (name, age) VALUES ('Bob',   69);"#);
    /// # conn.execute(stmt).unwrap();
    /// let age = String::from("42 or 1=1; --");  // input by attcker
    /// let sql = conn.prepare("SELECT name FROM users WHERE age < ") + unsafe { conn.without_escape(&age) };
    /// assert_eq!(sql.actual_sql(), "SELECT name FROM users WHERE age < 42 or 1=1; --");
    /// assert!(conn.rows(&sql).is_ok());
    /// ```
    ///
    /// # Safety
    ///
    /// - Use trusted values
    /// - Use in an environment where SQL injection does not occur
    pub unsafe fn without_escape<T: ?Sized + ToString>(&self, query: &T) -> OwString {
        OwString::new(query)
    }

    /// It is guaranteed to be a signed 64-bit integer without quotation.
    ///
    /// # Examples
    ///
    /// ```
    /// # let conn = exowsql::sqlite::open(":memory:").unwrap();
    /// conn.int(42);              // ok
    /// conn.int("42");            // ok
    /// conn.int("42 or 1=1; --"); // error
    /// ```
    pub fn int<T: Clone + ToString>(&self, value: T) -> Result<OwString, &str> {
        let value = value.to_string();
        if value.parse::<i64>().is_ok() {
            Ok(OwString::new(&value))
        } else {
            Err("not integer")
        }
    }

    /// Sets the error level.  
    /// The default value is [OwsqlErrorLevel](./enum.OwsqlErrorLevel.html)::Develop for debug builds and [OwsqlErrorLevel](./enum.OwsqlErrorLevel.html)::Release for release builds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use exowsql::OwsqlErrorLevel;
    /// # let mut conn = exowsql::sqlite::open(":memory:").unwrap();
    /// conn.error_level(OwsqlErrorLevel::Debug);
    /// ```
    pub fn error_level(&mut self, level: OwsqlErrorLevel) {
        self.error_level = level;
    }
}

