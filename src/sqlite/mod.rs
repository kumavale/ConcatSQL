//! `feature = "sqlite"` Interface to [SQLite](https://www.sqlite.org) of OverwriteSQL.

use std::path::Path;
use crate::Result;

#[macro_use]
mod parser;
mod connection;
mod token;
mod value;

/// This macro is a convenient way to pass named parameters to a statement.
///
/// ```ignore
/// let foo = 42;
/// let sql = conn.valid("SELECT {foo}, {foo2x} FROM bar;", params![ foo, "foo2x" => foo * 2 ]);
/// ```
#[macro_export]
macro_rules! params {
    () => {};
    (@to_pair $name:expr => $value:expr) => (
        (std::string::String::from($name), $crate::sqlite::value::Value::from($value))
    );
    (@to_pair $name:ident) => (
        (std::string::String::from(stringify!($name)), $crate::sqlite::value::Value::from($name))
    );
    (@expand $vec:expr;) => {};
    (@expand $vec:expr; $name:expr => $value:expr, $($tail:tt)*) => {
        $vec.push(params!(@to_pair $name => $value));
        params!(@expand $vec; $($tail)*);
    };
    (@expand $vec:expr; $name:expr => $value:expr $(, $tail:tt)*) => {
        $vec.push(params!(@to_pair $name => $value));
        params!(@expand $vec; $($tail)*);
    };
    (@expand $vec:expr; $name:ident, $($tail:tt)*) => {
        $vec.push(params!(@to_pair $name));
        params!(@expand $vec; $($tail)*);
    };
    (@expand $vec:expr; $name:ident $(, $tail:tt)*) => {
        $vec.push(params!(@to_pair $name));
        params!(@expand $vec; $($tail)*);
    };
    ($i:ident, $($tail:tt)*) => {
        {
            let mut output = std::vec::Vec::new();
            params!(@expand output; $i, $($tail)*);
            output
        }
    };
    ($i:expr => $($tail:tt)*) => {
        {
            let mut output = std::vec::Vec::new();
            params!(@expand output; $i => $($tail)*);
            output
        }
    };
    ($i:ident) => {
        {
            let mut output = std::vec::Vec::new();
            params!(@expand output; $i);
            output
        }
    }
}

/// Output type of params macro.
type Params = Vec<(String, value::Value)>;

pub use self::connection::Connection;

/// Open a read-write connection to a new or existing database.
pub fn open<T: AsRef<Path>>(path: T) -> Result<Connection> {
    Connection::open(path)
}


#[cfg(test)]
mod tests {
    use crate::sqlite::value::Value;

    #[test]
    fn sqlite_open() {
        let _conn = crate::sqlite::open(":memory:").unwrap();
        #[cfg(unix)]
        let _conn = crate::sqlite::open("/tmp/tmp.db").unwrap();
    }

    #[test]
    #[should_panic = "failed to connect"]
    fn sqlite_open_failed() {
        use std::path::Path;
        let _conn = crate::sqlite::open(Path::new("/path/to/db")).unwrap();
    }

    #[test]
    #[allow(clippy::blacklisted_name)]
    fn params_macro() {
        let foo = 42;
        let bar = "bar";

        assert_eq!(vec![(String::from("foo"), Value::Int(42))], params![ foo ]);
        assert_eq!(vec![(String::from("foo"), Value::Int(42))], params![ foo, ]);
        assert_eq!(
            vec![
                (String::from("foo"), Value::Int(42)),
                (String::from("bar"), Value::String(String::from("bar"))),
            ],
            params![ foo, bar ]
        );
        assert_eq!(
            vec![
                (String::from("foo"), Value::Int(42)),
                (String::from("bar"), Value::String(String::from("bar"))),
            ],
            params![ foo, bar, ]
        );
        assert_eq!(
            vec![
                (String::from("foo"), Value::Int(42)),
                (String::from("bar"), Value::String(String::from("bar"))),
            ],
            params! { "foo" => foo, "bar" => bar }
        );
        assert_eq!(
            vec![
                (String::from("foo"), Value::Int(42)),
                (String::from("bar"), Value::String(String::from("bar"))),
            ],
            params! { "foo" => foo, "bar" => bar, }
        );
        assert_eq!(
            vec![
                (String::from("foo"), Value::Int(42)),
                (String::from("bar"), Value::String(String::from("bar"))),
            ],
            params! { foo, "bar" => bar }
        );
        assert_eq!(
            vec![
                (String::from("foo"), Value::Int(42)),
                (String::from("bar"), Value::String(String::from("bar"))),
            ],
            params! { "foo" => foo, bar }
        );
        assert_eq!(
            vec![
                (String::from("foo"), Value::Int(42)),
                (String::from("bar"), Value::String(String::from("bar"))),
            ],
            params! { foo, "bar" => bar, }
        );
        assert_eq!(
            vec![
                (String::from("foo"), Value::Int(42)),
                (String::from("bar"), Value::String(String::from("bar"))),
            ],
            params! { "foo" => foo, bar, }
        );
    }
}
