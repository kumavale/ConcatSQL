extern crate postgres_sys as postgres;

use postgres::{Client, NoTls};

use std::collections::HashSet;
use std::cell::RefCell;
use std::fmt;

use crate::Result;
use crate::OwsqlConn;
use crate::bidimap::BidiMap;
use crate::error::{OwsqlError, OwsqlErrorLevel};
use crate::constants::OW_MINIMUM_LENGTH;
use crate::overwrite::{IntoInner, overwrite_new};
use crate::serial::SerialNumber;
use crate::parser::*;
use crate::row::Row;

/// A database connection for PostgreSQL.
pub struct PostgreSQLConnection {
    conn:                   RefCell<postgres::Client>,
    params:                 String, // tmp
    allowlist:              HashSet<String>,
    serial_number:          RefCell<SerialNumber>,
    ow_len_range:           (usize, usize),
    pub(crate) overwrite:   RefCell<BidiMap<String, String>>,
    pub(crate) error_msg:   RefCell<BidiMap<OwsqlError, String>>,
    pub(crate) error_level: OwsqlErrorLevel,
}

impl PartialEq for PostgreSQLConnection {
    fn eq(&self, other: &Self) -> bool {
        self.conn.as_ptr() == other.conn.as_ptr()
    }
}

impl fmt::Debug for PostgreSQLConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PostgreSQLConnection")
            .field("params", &self.params)
            .field("error_level", &self.error_level)
            .finish()
    }
}

impl PostgreSQLConnection {
    /// Open a read-write connection to a new or existing database.
    #[inline]
    pub fn open(params: &str) -> Result<Self> {
        let conn = match Client::connect(&params, NoTls) {
            Ok(conn) => conn,
            Err(e) => return Err(OwsqlError::new(format!("failed to open: {}", e))),
        };

        Ok(PostgreSQLConnection {
            conn:          RefCell::new(conn),
            params:        params.to_string(),
            allowlist:     HashSet::new(),
            serial_number: RefCell::new(SerialNumber::default()),
            ow_len_range:  (OW_MINIMUM_LENGTH, OW_MINIMUM_LENGTH),
            overwrite:     RefCell::new(BidiMap::new()),
            error_msg:     RefCell::new(BidiMap::new()),
            error_level:   OwsqlErrorLevel::default(),
            //error_level:   dbg!(OwsqlErrorLevel::Debug), // for develop
        })
    }

    /// Execute a statement without processing the resulting rows if any.
    ///
    /// # Examples
    ///
    /// ```
    /// # let conn = owsql::postgres::open("host=localhost user=postgres password=postgres").unwrap();
    /// # let stmt = conn.ow(r#"CREATE TEMPORARY TABLE users (name TEXT, id INTEGER);
    /// #                       INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #                       INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
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
        };

        match self.conn.borrow_mut().batch_execute(&query) {
            Ok(_) => Ok(()),
            Err(e) => self.err("exec error", &e.to_string()),
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
    /// # let conn = owsql::postgres::open("host=localhost user=postgres password=postgres").unwrap();
    /// # let stmt = conn.ow(r#"CREATE TEMPORARY TABLE users (name TEXT, id INTEGER);
    /// #                       INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #                       INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
    /// # conn.execute(stmt).unwrap();
    /// let sql = conn.ow(r#"SELECT * FROM users;"#);
    /// conn.iterate(&sql, |pairs| {
    ///     for (column, value) in pairs {
    ///         println!("{} = {}", column, value.as_ref().unwrap());
    ///     }
    ///     true
    /// }).unwrap();
    /// ```
    #[inline]
    pub fn iterate<T: AsRef<str>, F>(&self, query: T, mut callback: F) -> Result<()>
        where
            F: FnMut(&[(String, Option<String>)]) -> bool,
    {
        let query = match self.convert_to_valid_syntax(query.as_ref()) {
            Ok(query) => query,
            Err(e) => if self.error_level == OwsqlErrorLevel::AlwaysOk {
                return Ok(());
            } else {
                return Err(e);
            },
        };

        let mut conn = self.conn.borrow_mut();
        let statement = match conn.prepare(&query) {
            Ok(stmt) => stmt,
            Err(e) => return self.err("exec error", &e.to_string()),
        };

        let rows = match conn.query(&statement, &[]) {
            Ok(result) => result,
            Err(e) => return self.err("exec error", &e.to_string()),
        };

        let mut pairs = Vec::new();
        for row in rows {
            for col in row.columns() {
                //pairs.push((col.name().to_string(), row.try_get::<&str, String>(col.name()).ok()));
                let value = if let Ok(v) = row.try_get::<&str, String>(col.name()) {
                    Some(v)
                } else if let Ok(v) = row.try_get::<&str, i32>(col.name()) {
                    Some(v.to_string())
                } else {
                    None
                };

                pairs.push((col.name().to_string(), value));
            }
        }
        if !pairs.is_empty() && !callback(&pairs) {
            return self.err("exec error", "query aborted");
        }

        Ok(())
    }

    /// Execute a statement and returns the rows.
    ///
    /// # Examples
    ///
    /// ```
    /// # let conn = owsql::postgres::open("host=localhost user=postgres password=postgres").unwrap();
    /// # let stmt = conn.ow(r#"CREATE TEMPORARY TABLE users (name TEXT, id INTEGER);
    /// #                       INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #                       INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
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
            for (column, value) in pairs.iter() {
                row.insert(column.to_string(), value.as_ref().map(|v| v.to_string()));
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
    /// # let conn = owsql::postgres::open("host=localhost user=postgres password=postgres").unwrap();
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
    /// ```rust
    /// # let conn = owsql::postgres::open("host=localhost user=postgres password=postgres").unwrap();
    /// # let name = "bar";
    /// conn.ow("where name = 'foo' OR name = '") + name + &conn.ow("';");
    /// # /*
    ///                                       ^                      ^
    /// # */
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// # let conn = owsql::postgres::open("host=localhost user=postgres password=postgres").unwrap();
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
        let s = format!("'{}'", single_quotaion_and_backslash_escape(&value.to_string()));
        let result = self.check_valid_literal(&s);
        match result {
            Ok(_) => {
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
    /// # let mut conn = owsql::postgres::open("host=localhost user=postgres password=postgres").unwrap();
    /// # let stmt = conn.ow(r#"CREATE TEMPORARY TABLE users (name TEXT, id INTEGER);
    /// #                       INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #                       INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
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
    /// # let mut conn = owsql::postgres::open("host=localhost user=postgres password=postgres").unwrap();
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
    /// # let mut conn = owsql::postgres::open("host=localhost user=postgres password=postgres").unwrap();
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
}

impl OwsqlConn for crate::postgres::PostgreSQLConnection {
    #[inline]
    fn err(&self, err_msg: &str, detail_msg: &str) -> Result<(), OwsqlError> {
        match self.error_level {
            OwsqlErrorLevel::AlwaysOk => Ok(()),
            OwsqlErrorLevel::Release  => Err(OwsqlError::AnyError),
            OwsqlErrorLevel::Develop  => Err(OwsqlError::new(&err_msg)),
            OwsqlErrorLevel::Debug    => Err(OwsqlError::new(&format!("{}: {}", err_msg, detail_msg))),
        }
    }
}
