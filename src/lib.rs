//! # ConcatSQL
//! `concatsql` is a secure library for PostgreSQL, MySQL and SQLite.
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
pub use crate::error::{Error, ErrorLevel};
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
    pub use crate::row::Row;
    pub use crate::{sanitize_like, prepare, int};
    pub use crate::{WrapString, Wrap};
}

/// A typedef of the result returned by many methods.
pub type Result<T, E = crate::error::Error> = std::result::Result<T, E>;

/// Prepare a SQL statement for execution.
///
/// # Examples
///
/// ```
/// use concatsql::prepare;
/// # let conn = concatsql::sqlite::open(":memory:").unwrap();
/// # let stmt = prepare!(r#"CREATE TABLE users (name TEXT, id INTEGER);
/// #               INSERT INTO users (name, id) VALUES ('Alice', 42);
/// #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
/// # conn.execute(stmt).unwrap();
/// for name in ["Alice", "Bob"].iter() {
///     let stmt = prepare!("INSERT INTO users (name) VALUES (") + &name + prepare!(")");
///     conn.execute(stmt).unwrap();
/// }
/// ```
///
/// # Failure
///
/// If you take a value other than `&'static str` as an argument, it will fail.
///
/// ```compile_fail
/// # use concatsql::prepare;
/// let passwd = String::from("'' or 1=1; --");
/// prepare!("SELECT * FROM users WHERE passwd=") + prepare!(&passwd); // shouldn't compile!
/// ```
///
/// # Panics
///
/// Panic if you have incomplete single or double quotes.
///
/// panics:
///
/// ```should_panic
/// # use concatsql::prepare;
/// # let id = 42;
/// prepare!("SELECT * FROM users WHERE id='") + id + prepare!("'");
/// prepare!("INSERT INTO msg VALUES ('I'm cat.')");
/// ```
///
/// correct:
///
/// ```
/// # use concatsql::prepare;
/// # let id = 42;
/// prepare!("SELECT * FROM users WHERE id=") + id;
/// prepare!("INSERT INTO msg VALUES ('I''m cat.')");
/// prepare!("INSERT INTO msg VALUES (\"I'm cat.\")");
/// ```
#[macro_export]
macro_rules! prepare {
    () => { concatsql::WrapString::init("") };
    ($query:expr) => {
        {
            static INITIAL_CHECK: std::sync::Once = std::sync::Once::new();
            INITIAL_CHECK.call_once(|| concatsql::check_valid_literal($query).unwrap());
            concatsql::WrapString::init($query)
        }
    };
}

/// It is guaranteed to be a signed 64-bit integer without quotation.
///
/// # Examples
///
/// ```
/// use concatsql::int;
/// # let conn = concatsql::sqlite::open(":memory:").unwrap();
/// assert!(int!(42).is_ok());
/// assert!(int!("42").is_ok());
/// assert!(int!("42 or 1=1; --").is_err());
/// ```
#[macro_export]
macro_rules! int {
    ($query:expr) => { concatsql::WrapString::int($query) };
}

