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
//!     let sql = prep!("SELECT name FROM users WHERE age = ") + &age;
//!     // At runtime it will be transformed into a query like
//!     assert_eq!(sql.actual_sql(), "SELECT name FROM users WHERE age = '42'");
//!     for row in conn.rows(&sql).unwrap() {
//!         assert_eq!(row.get(0).unwrap(),      "Alice");
//!         assert_eq!(row.get("name").unwrap(), "Alice");
//!     }
//!
//!     let age = String::from("42 OR 1=1; --");  // user input
//!     let sql = prep!("SELECT name FROM users WHERE age = ") + &age;
//!     // At runtime it will be transformed into a query like
//!     assert_eq!(sql.actual_sql(), "SELECT name FROM users WHERE age = '42 OR 1=1; --'");
//!     conn.iterate(&sql, |_| { unreachable!() }).unwrap();
//! }
//! ```

#![allow(clippy::needless_doctest_main)]
#![cfg_attr(docsrs, feature(doc_cfg))]

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

pub use crate::connection::{Connection, SafeStr};
pub use crate::error::{Error, ErrorLevel};
pub use crate::row::{Row, Get, FromSql};
pub use crate::parser::{html_special_chars, _sanitize_like, check_valid_literal, invalid_literal};
pub use crate::wrapstring::{WrapString, ToWrapString, Num};

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

    pub use crate::connection::{Connection, SafeStr};
    pub use crate::row::{Row, Get, FromSql};
    pub use crate::{sanitize_like, prep};
    pub use crate::wrapstring::{WrapString, ToWrapString};
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
/// # use concatsql::prelude::*;
/// let passwd = String::from("'' or 1=1; --");
/// prep!("SELECT * FROM users WHERE passwd=") + prep!(&passwd); // shouldn't compile!
/// ```
///
/// # Panics
///
/// **SQL injection successful if you have incomplete single or double quotes.**  
/// Panic when debug builds and display warning messages when release builds.  
///
/// ```should_panic
/// # use concatsql::prelude::*;
/// # let id = 42;
/// prep!("SELECT * FROM users WHERE id='") + id + prep!("'");
/// prep!("INSERT INTO msg VALUES ('I'm cat.')");
/// assert_eq!((prep!("WHERE passwd='") + " or 1=1; --" + prep!("'")).actual_sql(), "WHERE passwd='' or 1=1; --''"); // When release builds
/// ```
///
/// # Safety
///
/// ```
/// # use concatsql::prelude::*;
/// # let id = 42;
/// prep!("SELECT * FROM users WHERE id=") + id;
/// prep!("INSERT INTO msg VALUES ('I''m cat.')");
/// prep!("INSERT INTO msg VALUES (\"I'm cat.\")");
/// ```
#[macro_export]
macro_rules! prep {
    () => { concatsql::WrapString::init("") };
    ($query:expr) => {
        {
            // I want to make an error at compile time...
            static INITIAL_CHECK: std::sync::Once = std::sync::Once::new();
            INITIAL_CHECK.call_once(|| {
                if let Err(detail) = concatsql::check_valid_literal($query) {
                    eprintln!("{}{}:{}", concatsql::invalid_literal(), file!(), line!());
                    eprintln!("{}", detail.to_string());

                    #[cfg(debug_assertions)]
                    panic!("invalid literal");
                }
            });
            concatsql::WrapString::init($query)
        }
    };
}

