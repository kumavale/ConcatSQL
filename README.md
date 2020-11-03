# OverwriteSQL

[![Actions Status](https://github.com/kumavale/OverwriteSQL/workflows/CI/badge.svg)](https://github.com/kumavale/OverwriteSQL/actions)
[![Crates.io](https://img.shields.io/crates/v/owsql.svg)](https://crates.io/crates/owsql)
[![license](https://img.shields.io/badge/license-MIT-blue.svg?style=flat)](LICENSE)
[![Documentation](https://docs.rs/owsql/badge.svg)](https://docs.rs/owsql)
  

OverwriteSQL(`owsql`) is a secure SQL database library I'm currently developing as project for my graduation work.  
You can use string concatenation to prevent SQL injection.  

Supported databases:
- PostgreSQL
- MySQL
- SQLite

## Installation

You can configure the database backend in `Cargo.toml`:

```toml
[dependencies]
owsql = { version = "<version>", features = ["<postgres|mysql|sqlite>"] }
```

## Examples

### Normal value

```rust
let conn = owsql::sqlite::open(":memory:").unwrap();
let age = String::from("50");
let sql = conn.ow("SELECT name FROM users WHERE") + &conn.ow("age <") + &age;
// sql = " OWSQLa81259BW1UpAsw3FqI39v6YY  OWSQLxOBx4vbxPQ5dUMkdPHN5iIux 50"
assert_eq!(conn.actual_sql(&sql).unwrap(), "SELECT name FROM users WHERE age < '50' ");
for (i, row) in conn.rows(&sql).unwrap().iter().enumerate() {
    assert_eq!(row.get("name").unwrap(), "Alice");
}
```

### Illegal value

```rust
let conn = owsql::sqlite::open(":memory:").unwrap();
let age = String::from("50 or 1=1; --");
let sql = conn.ow("SELECT name FROM users WHERE") + &conn.ow("age <") + &age;
// sql = " OWSQLbPAGSyVagjC9Ui5ZlkJprFpA OWSQLiiFkB2vqASM8I3JLa9O5vOOs 50 or 1=1; --"
assert_eq!(conn.actual_sql(&sql).unwrap(), "SELECT name FROM users WHERE age < '50 or 1=1; --' ");
assert!(conn.rows(&sql).is_err());
```

### If you did not use the conn.ow() method

```rust
let conn = owsql::sqlite::open(":memory:").unwrap();
let age = String::from("50 or 1=1; --");
let sql = "SELECT name FROM users WHEREage < " + &age;
assert_eq!(conn.actual_sql(&sql).unwrap(), "'SELECT name FROM users WHERE age < 50 or 1=1; --' ");
assert!(conn.rows(&sql).is_err());
```

### conn.ow(\<String\>)

>> ```rust
>> pub fn ow<T: ?Sized + std::string::ToString>(&self, s: &'static T) -> String;
>> ```

cannot compile

```rust
let conn = owsql::sqlite::open(":memory:").unwrap();
let age = String::from("50 or 1=1; --");
let sql = conn.ow("SELECT name FROM users WHERE age <") + &conn.ow(&age);  // error
```

## License

MIT

