extern crate mysql_sys as mysql;
use mysql::{Opts, Conn};
use mysql::prelude::*;

use std::collections::HashSet;
use std::cell::RefCell;
use std::fmt;

use crate::Result;
use crate::bidimap::BidiMap;
use crate::error::{OwsqlError, OwsqlErrorLevel};
use crate::constants::OW_MINIMUM_LENGTH;
use crate::overwrite::{IntoInner, overwrite_new};
use crate::serial::SerialNumber;
use crate::parser::{escape_for_allowlist, single_quotaion_escape};
use super::row::MysqlRow;

/// A database connection for MySQL.
pub struct MysqlConnection {
    conn:                   RefCell<mysql::Conn>,
    allowlist:              HashSet<String>,
    serial_number:          RefCell<SerialNumber>,
    ow_len_range:           (usize, usize),
    pub(crate) overwrite:   RefCell<BidiMap<String, String>>,
    pub(crate) error_msg:   RefCell<BidiMap<OwsqlError, String>>,
    pub(crate) error_level: OwsqlErrorLevel,
}

impl PartialEq for MysqlConnection {
    fn eq(&self, other: &Self) -> bool {
        self.conn.as_ptr() == other.conn.as_ptr()
    }
}

impl fmt::Debug for MysqlConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MysqlConnection")
            .field("conn", &self.conn)
            .field("error_level", &self.error_level)
            .finish()
    }
}

impl MysqlConnection {
    /// Open a read-write connection to a new or existing database.
    #[inline]
    pub fn open(url: &str) -> Result<Self> {
        let opts = match Opts::from_url(&url) {
            Ok(opts) => opts,
            Err(e) => return Err(OwsqlError::new(format!("failed to open: {}", e))),
        };

        let conn = match Conn::new(opts) {
            Ok(conn) => conn,
            Err(e) => return Err(OwsqlError::new(format!("failed to open: {}", e))),
        };

        Ok(MysqlConnection {
            conn:          RefCell::new(conn),
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
    /// # let mut conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
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

        match self.conn.borrow_mut().query_drop(&query) {
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
    /// # let mut conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
    /// # let stmt = conn.ow(r#"CREATE TEMPORARY TABLE users (name TEXT, id INTEGER);
    /// #                       INSERT INTO users (name, id) VALUES ('Alice', 42);
    /// #                       INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
    /// # conn.execute(stmt).unwrap();
    /// let sql = conn.ow(r#"SELECT * FROM users;"#);
    /// conn.iterate(&sql, |pairs| {
    ///     for (column, value) in pairs.iter() {
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
        let mut result = match conn.query_iter(&query) {
            Ok(result) => result,
            Err(e) => return self.err("exec error", &e.to_string()),
        };

        while let Some(result_set) = result.next_set() {
            let result_set = match result_set {
                Ok(result_set) => result_set,
                Err(e) => return self.err("exec error", &e.to_string()),
            };
            let mut pairs = Vec::with_capacity(result_set.affected_rows() as usize);

            for row in result_set {
                let row = match row {
                    Ok(row) => row,
                    Err(e) => return self.err("exec error", &e.to_string()),
                };

                for (i, col) in row.columns().iter().enumerate() {
                    pairs.push((col.name_str().to_string(), row.get(i)));
                }

            }

            if !callback(&pairs) {
                return self.err("exec error", "query aborted");
            }
        }

        Ok(())
    }

    /// Execute a statement and returns the rows.
    ///
    /// # Examples
    ///
    /// ```
    /// # let mut conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
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
    pub fn rows<T: AsRef<str>>(&self, query: T) -> Result<Vec<MysqlRow>> {
        let mut rows: Vec<MysqlRow> = Vec::new();

        self.iterate(query, |pairs| {
            let mut row = MysqlRow::new();
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
    /// # let mut conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
    /// let select = conn.ow("SELECT");
    /// let oreilly = conn.ow("O'Reilly");
    ///// let oreilly_unhtmlescape = unsafe { conn.ow_without_html_escape("O'Reilly") };
    /// assert_eq!(conn.actual_sql(&select).unwrap(), "SELECT ");
    /// assert_eq!(conn.actual_sql("SELECT").unwrap(), "'SELECT' ");
    /// assert_eq!(conn.actual_sql(&oreilly), Err(OwsqlError::Message("invalid literal".to_string())));
    /// assert_eq!(conn.actual_sql("O'Reilly").unwrap(), "'O&#39;Reilly' ");
    ///// assert_eq!(conn.actual_sql(&oreilly_unhtmlescape).unwrap(), "'O''Reilly' ");
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
    /// # let mut conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
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
    /// # let mut conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
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

