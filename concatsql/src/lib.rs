//! # ConcatSQL
//!
//! `concatsql` is a secure library for PostgreSQL, MySQL and SQLite.  
//! Unlike other libraries, you can use string concatenation to prevent SQL injection.  
//!
//! ```rust
//! use concatsql::prelude::*;
//!
//! fn main() {
//!     let conn = concatsql::sqlite::open(":memory:").unwrap();
//!     conn.execute(r#"
//!             CREATE TABLE users (name TEXT, age INTEGER);
//!             INSERT INTO users (name, age) VALUES ('Alice', 42);
//!             INSERT INTO users (name, age) VALUES ('Bob',   69);
//!     "#).unwrap();
//!
//!     let age = String::from("42");  // user input
//!     let sql = query!("SELECT name FROM users WHERE age = {age}");
//!     // At runtime it will be transformed into a query like
//!     assert_eq!(sql.simulate(), "SELECT name FROM users WHERE age = '42'");
//!     for row in conn.rows(&sql).unwrap() {
//!         assert_eq!(row.get(0).unwrap(),      "Alice");
//!         assert_eq!(row.get("name").unwrap(), "Alice");
//!     }
//!
//!     let age = String::from("42 OR 1=1; --");  // user input
//!     let sql = query!("SELECT name FROM users WHERE age = {age}");
//!     // At runtime it will be transformed into a query like
//!     assert_eq!(sql.simulate(), "SELECT name FROM users WHERE age = '42 OR 1=1; --'");
//!     conn.iterate(&sql, |_| { unreachable!() }).unwrap();
//! }
//! ```

#![allow(clippy::needless_doctest_main)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod connection;
mod error;
mod parser;
mod row;
mod value;
mod wrapstring;

#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
pub mod mysql;
#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
pub mod postgres;
#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
pub mod sqlite;

pub use crate::connection::{without_escape, Connection};
pub use crate::error::{Error, ErrorLevel};
pub use crate::parser::{_sanitize_like, html_special_chars, invalid_literal};
pub use crate::row::{FromSql, Get, Row};
pub use crate::value::{ToValue, Value};
pub use crate::wrapstring::{IntoWrapString, WrapString};

pub use concatsql_macro::query;

pub mod prelude {
    //! Re-exports important traits and types.

    #[cfg(feature = "mysql")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
    pub use crate::mysql;
    #[cfg(feature = "postgres")]
    #[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
    pub use crate::postgres;
    #[cfg(feature = "sqlite")]
    #[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
    pub use crate::sqlite;

    pub use crate::connection::{without_escape, Connection};
    pub use crate::row::{FromSql, Get, Row};
    pub use crate::value::{ToValue, Value};
    pub use crate::wrapstring::WrapString;
    pub use crate::{params, sanitize_like};
    pub use concatsql_macro::query;
}

/// A typedef of the result returned by many methods.
pub type Result<T, E = crate::error::Error> = std::result::Result<T, E>;

/// Prepare a SQL statement for execution.
///
/// # Examples
///
/// ```
/// use concatsql::prep;
/// # let conn = concatsql::sqlite::open(":memory:").unwrap();
/// # let stmt = prep!(r#"CREATE TABLE users (name TEXT, id INTEGER);
/// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
/// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
/// # conn.execute(stmt).unwrap();
/// for name in ["Alice", "Bob"].iter() {
///     let stmt = prep!("INSERT INTO users (name) VALUES (") + name + prep!(")");
///     conn.execute(stmt).unwrap();
/// }
/// ```
///
/// # Failure
///
/// If you take a value other than `&'static str` as an argument, it will fail.
///
/// ```compile_fail
/// # use concatsql::prep;
/// let passwd = String::from("'' or 1=1; --");
/// prep!("SELECT * FROM users WHERE passwd=") + prep!(&passwd); // shouldn't compile!
/// ```
///
/// # Safety
///
/// ```
/// # use concatsql::prep;
/// prep!("SELECT * FROM users WHERE id=") + 42;
/// prep!("INSERT INTO msg VALUES ('I''m cat.')");
/// prep!("INSERT INTO msg VALUES (\"I'm cat.\")");
/// prep!("INSERT INTO msg VALUES (") + "I'm cat." + prep!(")");
/// ```
#[deprecated(note = "please use `query!` instead")]
#[allow(deprecated)]
#[macro_export]
macro_rules! prep {
    () => {
        $crate::WrapString::null()
    };
    ($query:expr) => {
        $crate::WrapString::init($query)
    };
}

/// Prepare a SQL statement for execution.
///
/// # Examples
///
/// ```
/// use concatsql::prep;
/// # let conn = concatsql::sqlite::open(":memory:").unwrap();
/// # let stmt = prep!(r#"CREATE TABLE users (name TEXT, id INTEGER);
/// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
/// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
/// # conn.execute(stmt).unwrap();
/// for name in ["Alice", "Bob"].iter() {
///     let stmt = prep("INSERT INTO users (name) VALUES (") + name + prep(")");
///     conn.execute(stmt).unwrap();
/// }
/// ```
///
/// # Failure
///
/// If you take a value other than `&'static str` as an argument, it will fail.
///
/// ```compile_fail
/// # use concatsql::prep;
/// let passwd = String::from("'' or 1=1; --");
/// prep("SELECT * FROM users WHERE passwd=") + prep(&passwd); // shouldn't compile!
/// ```
///
/// # Safety
///
/// ```
/// # use concatsql::prep;
/// prep("SELECT * FROM users WHERE id=") + 42;
/// prep("INSERT INTO msg VALUES ('I''m cat.')");
/// prep("INSERT INTO msg VALUES (\"I'm cat.\")");
/// prep("INSERT INTO msg VALUES (") + "I'm cat." + prep(")");
/// ```
#[inline]
#[deprecated(note = "please use `query!` instead")]
#[allow(deprecated)]
pub fn prep(query: &'static str) -> WrapString {
    WrapString::init(query)
}

/// A macro making it more convenient to pass heterogeneous lists
/// of parameters as a `&[&dyn ToValue]`.
///
/// # Example
///
/// ```
/// # use concatsql::prelude::*;
/// # use concatsql::prep;
/// let sql = prep("VALUES(") + params![42i32,"Alice"] + prep(")");
/// assert_eq!(sql.simulate(), "VALUES(42,'Alice')");
/// ```
#[macro_export]
macro_rules! params {
    ( $( $param:expr ),+ $(,)? ) => {
        &[ $(&$param as &dyn $crate::ToValue),+ ] as &[&dyn $crate::ToValue]
    };
}
