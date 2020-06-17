//! # OverwriteSQL
//! `owsql` is a secure library for PostgreSQL, MySQL and SQLite.
//!
//! ## Example
//!
//! Open a connection of SQLite, create a table, and insert some rows:
//!
//! ```
//! let mut conn = owsql::sqlite::open(":memory:").unwrap();
//! let stmt = conn.ow(r#"CREATE TABLE users (name TEXT, age INTEGER);
//!               INSERT INTO users (name, age) VALUES ('Alice', 42);
//!               INSERT INTO users (name, age) VALUES ('Bob', 69);"#);
//! conn.execute(stmt).unwrap();
//! ```
//!
//! Select some rows and process them one by one as plain text:
//!
//! ```
//! # let mut conn = owsql::sqlite::open(":memory:").unwrap();
//! # let stmt = conn.ow(r#"CREATE TABLE users (name TEXT, age INTEGER);
//! #               INSERT INTO users (name, age) VALUES ('Alice', 42);
//! #               INSERT INTO users (name, age) VALUES ('Bob', 69);"#);
//! # conn.execute(stmt).unwrap();
//! let age = "50";
//! let sql = conn.ow("SELECT * FROM users WHERE age > ") + age;
//! conn.iterate(sql, |pairs| {
//!     for &(column, value) in pairs.iter() {
//!         println!("{} = {}", column, value.unwrap());
//!     }
//!     true
//! }).unwrap();
//! ```


mod bidimap;

#[cfg(feature = "sqlite")]
pub mod sqlite;

