# ExOverwriteSQL

[![Actions Status](https://github.com/kumavale/ExOverwriteSQL/workflows/CI/badge.svg)](https://github.com/kumavale/ExOverwriteSQL/actions)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat)](LICENSE)
  

ExOverwriteSQL(`exowsql`) is a secure SQL database library I'm currently developing as project for my graduation work.  
You can use string concatenation to prevent SQL injection.  

Supported databases:
- PostgreSQL
- MySQL
- SQLite

## Installation

You can configure the database backend in `Cargo.toml`:

```toml
[dependencies]
owsql = { git = "https://github.com/kumavale/ExOverwriteSQL", features = ["<postgres|mysql|sqlite>"] }
```

## Examples

### Normal value

```rust
use exowsql::{prepare, bind};
let conn = exowsql::sqlite::open(":memory:").unwrap();
let id     = String::from("42");
let passwd = String::from("pass");
let sql = prepare("SELECT name FROM users WHERE id=") + bind(&id) + prepare(" AND passwd='") + bind(passwd) + prepare("';");
assert_eq!(exowsql::actual_sql(&sql), "SELECT name FROM users WHERE id=42 AND passwd='pass';");
for (i, row) in conn.rows(&sql).unwrap().iter().enumerate() {
    assert_eq!(row.get("name").unwrap(), "Alice");
}
```

### Illegal value

```rust
use exowsql::{prepare, bind};
let conn = exowsql::sqlite::open(":memory:").unwrap();
let id     = String::from("42");
let passwd = String::from("pass");
let sql = prepare("SELECT name FROM users WHERE id=") + bind(&id) + prepare(" AND passwd='") + bind(passwd) + prepare("';");
assert_eq!(conn.actual_sql(&sql), "SELECT name FROM users WHERE id=42 AND passwd=''' or 1=1; --';");
for (i, row) in conn.rows(&sql).unwrap().iter().enumerate() {
    unreachable!();
}
```

### If you did not use the exowsql::static function

cannot compile

```rust
let conn = exowsql::sqlite::open(":memory:").unwrap();
let id     = String::from("42");
let passwd = String::from("' or 1=1; --");
let sql = "SELECT name FROM users WHERE id=" + &id + " AND passwd='" + &passwd + "';";
conn.execute(&sql).unwrap();  // error
```

### exowsql::prepare(\<String\>)

cannot compile

>> ```rust
>> pub const fn prepare(query: &'static str) -> OwString;
>> ```

```rust
let conn = exowsql::sqlite::open(":memory:").unwrap();
let age = String::from("50 or 1=1; --");
let sql = exowsql::prepare("SELECT name FROM users WHERE age < ") + exowsql::prepare(&age);  // error
```

## License

MIT

