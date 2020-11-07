//! # OverwriteSQL
//! `owsql` is a secure library for PostgreSQL, MySQL and SQLite.  
//! Unlike other libraries, you can use string concatenation to prevent SQL injection.  
//!
//! ```rust
//! # use concatsql::prepare;
//! # let conn = concatsql::sqlite::open(":memory:").unwrap();
//! # let stmt = prepare!(r#"CREATE TABLE users (name TEXT, id INTEGER);
//! #               INSERT INTO users (name, id) VALUES ('Alice', 42);
//! #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
//! # conn.execute(stmt).unwrap();
//! let id_input = "42 OR 1=1; --";
//! let sql = prepare!("SELECT name FROM users WHERE id = ") + id_input;
//! // At runtime it will be transformed into a query like
//! // "SELECT name FROM users WHERE id = '42 OR 1=1; --'".
//! # conn.iterate(&sql, |_| { true }).unwrap();
//! ```
//!
//! ## Example
//!
//! Open a connection of SQLite, create a table, and insert some rows:
//!
//! ```rust
//! # use concatsql::prepare;
//! let conn = concatsql::sqlite::open(":memory:").unwrap();
//! let stmt = prepare!(r#"CREATE TABLE users (name TEXT, age INTEGER);
//!               INSERT INTO users (name, age) VALUES ('Alice', 42);
//!               INSERT INTO users (name, age) VALUES ('Bob', 69);"#);
//! conn.execute(&stmt).unwrap();
//! ```
//!
//! Select some rows and process them one by one as plain text:
//!
//! ```rust
//! # use concatsql::prepare;
//! # let conn = concatsql::sqlite::open(":memory:").unwrap();
//! # let stmt = prepare!(r#"CREATE TABLE users (name TEXT, age INTEGER);
//! #               INSERT INTO users (name, age) VALUES ('Alice', 42);
//! #               INSERT INTO users (name, age) VALUES ('Bob', 69);"#);
//! # conn.execute(stmt).unwrap();
//! let age = "50";
//! let sql = prepare!("SELECT * FROM users WHERE age > ") + age;
//! conn.iterate(&sql, |pairs| {
//!     for &(column, value) in pairs.iter() {
//!         println!("{} = {}", column, value.unwrap());
//!     }
//!     true
//! }).unwrap();
//! ```
//!
//! It can be executed after getting all the rows of the query:
//!
//! ```rust
//! # use concatsql::prepare;
//! # let conn = concatsql::sqlite::open(":memory:").unwrap();
//! # let stmt = prepare!(r#"CREATE TABLE users (name TEXT, age INTEGER);
//! #               INSERT INTO users (name, age) VALUES ('Alice', 42);
//! #               INSERT INTO users (name, age) VALUES ('Bob', 69);"#);
//! # conn.execute(stmt).unwrap();
//! let age = "50";
//! let sql = prepare!("SELECT * FROM users WHERE age > ") + age;
//! let rows = conn.rows(&sql).unwrap();
//! for row in rows.iter() {
//!     println!("name = {}", row.get("name").unwrap_or("NULL"));
//! }
//! ```


mod connection;
mod error;
mod parser;
mod row;
mod wrapstring;

#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
pub mod sqlite;
#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
pub mod mysql;
#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
pub mod postgres;

pub use crate::connection::Connection;
pub use crate::error::{ConcatsqlError, ConcatsqlErrorLevel};
pub use crate::row::Row;
pub use crate::parser::{html_special_chars, _sanitize_like, check_valid_literal};
pub use crate::wrapstring::{WrapString, Wrap};

pub mod prelude {
    //! Re-exports important traits and types.

    #[cfg(feature = "sqlite")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
    pub use crate::sqlite;
    #[cfg(feature = "mysql")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
    pub use crate::mysql;
    #[cfg(feature = "postgres")]
    #[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
    pub use crate::postgres;

    pub use crate::connection::Connection;
    pub use crate::error::{ConcatsqlError, ConcatsqlErrorLevel};
    pub use crate::row::Row;
    pub use crate::prepare;
    pub use crate::sanitize_like;
    pub use crate::{WrapString, Wrap};
}

/// A typedef of the result returned by many methods.
pub type Result<T, E = crate::error::ConcatsqlError> = std::result::Result<T, E>;

/// TODO docs
#[macro_export]
macro_rules! prepare {
    ($query:expr) => {
        {
            static INITIAL_CHECK: std::sync::Once = std::sync::Once::new();
            INITIAL_CHECK.call_once(|| concatsql::check_valid_literal($query).unwrap());
            concatsql::WrapString::new($query)
        }
    };
}

