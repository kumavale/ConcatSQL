//! # OverwriteSQL
//! `owsql` is a secure library for PostgreSQL, MySQL and SQLite.  
//! Unlike other libraries, you can use string concatenation to prevent SQL injection.  
//!
//! ```
//! # let mut conn = owsql::sqlite::open(":memory:").unwrap();
//! # let stmt = conn.ow(r#"CREATE TABLE users (name TEXT, id INTEGER);
//! #               INSERT INTO users (name, id) VALUES ('Alice', 42);
//! #               INSERT INTO users (name, id) VALUES ('Bob', 69);"#);
//! # conn.execute(stmt).unwrap();
//! let id_input = "42 OR 1=1; --";
//! let sql = conn.ow("SELECT name FROM users WHERE id = ") + id_input;
//! println!("[{}]", sql); // [ OWSQL47xyz6km0CfbRt0BA38Z2DxrleESyPPg4 42 OR 1=1; --]
//! // At runtime it will be transformed into a query like
//! // "SELECT name FROM users WHERE id = '42 OR 1=1; --'".
//! # conn.iterate(&sql, |_| { true }).unwrap();
//! ```
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
//! conn.execute(&stmt).unwrap();
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
//! ```
//! # let mut conn = owsql::sqlite::open(":memory:").unwrap();
//! # let stmt = conn.ow(r#"CREATE TABLE users (name TEXT, age INTEGER);
//! #               INSERT INTO users (name, age) VALUES ('Alice', 42);
//! #               INSERT INTO users (name, age) VALUES ('Bob', 69);"#);
//! # conn.execute(stmt).unwrap();
//! let age = "50";
//! let sql = conn.ow("SELECT * FROM users WHERE age > ") + age;
//! let rows = conn.rows(&sql).unwrap();
//! for row in rows.iter() {
//!     println!("name = {}", row.get("name").unwrap_or("NULL"));
//! }
//! ```


mod bidimap;
pub mod error;
pub mod constants;
#[doc(hidden)]
pub mod value;

#[cfg(feature = "sqlite")]
pub mod sqlite;
#[cfg(feature = "mysql")]
pub mod mysql;

/// A typedef of the result returned by many methods.
pub type Result<T, E = crate::error::OwsqlError> = std::result::Result<T, E>;

/// This macro is a convenient way to pass named parameters to a statement.
///
/// ```
/// # use owsql::params;
/// # let mut conn = owsql::sqlite::open(":memory:").unwrap();
/// let alice = "Alice";
/// let sql = conn.add_allowlist( params![ alice, "Bob" ] );
/// ```
#[macro_export]
macro_rules! params {
    ( $( $param:expr ),* ) => {
        {
            let mut temp_vec = Vec::new();
            $(
                temp_vec.push($crate::value::Value::from($param));
            )*
            temp_vec
        }
    };
}

/// Generate new overwrite string.
fn overwrite_new(serial: usize, range: (usize, usize)) -> String {
    use rand::{Rng, thread_rng};
    use rand::distributions::Alphanumeric;
    use std::cmp::Ordering;

    format!("OWSQL{}{}",
        thread_rng()
        .sample_iter(Alphanumeric)
        .take( match (range.0).cmp(&range.1) {
            Ordering::Equal   => range.0,
            Ordering::Less    => thread_rng().gen_range(range.0, range.1),
            Ordering::Greater => thread_rng().gen_range(range.1, range.0),
        })
        .collect::<String>(),
        serial.to_string())
}

pub trait IntoInner { fn into_inner(self) -> (usize, usize); }
impl IntoInner for usize                           { fn into_inner(self) -> (usize, usize) { (self, self) } }
impl IntoInner for std::ops::RangeInclusive<usize> { fn into_inner(self) -> (usize, usize) { self.into_inner() } }
impl IntoInner for std::ops::Range<usize> {
    fn into_inner(self) -> (usize, usize) {
        use std::cmp::Ordering;
        match (self.start).cmp(&self.end) {
            Ordering::Equal   => (self.start, self.end),
            Ordering::Less    => (self.start, self.end-1),
            Ordering::Greater => (self.start, self.end+1),
        }
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn into_inner() {
        use crate::IntoInner;
        assert_eq!(( 0,  0), (0).into_inner());
        assert_eq!((42, 42), (42).into_inner());
        assert_eq!(( 0, 31), (0..32).into_inner());
        assert_eq!(( 0, 32), (0..=32).into_inner());
        assert_eq!((64, 64), (64..64).into_inner());
        assert_eq!((64, 64), (64..=64).into_inner());
        assert_eq!((64, 33), (64..32).into_inner());
        assert_eq!((64, 32), (64..=32).into_inner());
    }
}

