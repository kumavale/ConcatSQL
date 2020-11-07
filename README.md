# ConcatSQL

[![Actions Status](https://github.com/kumavale/ConcatSQL/workflows/CI/badge.svg)](https://github.com/kumavale/ConcatSQL/actions)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat)](LICENSE)
  

ConcatSQL(`concatsql`) is a secure SQL database library I'm currently developing as project for my graduation work.  
You can use string concatenation to prevent SQL injection.  

Supported databases:
- PostgreSQL
- MySQL
- SQLite

## Installation

You can configure the database backend in `Cargo.toml`:

```toml
[dependencies]
concatsql = { git = "https://github.com/kumavale/ConcatSQL", features = ["<postgres|mysql|sqlite>"] }
```

## Examples

### Normal value

```rust
use concatsql::prepare;
let conn = concatsql::sqlite::open(":memory:").unwrap();
let id     = String::from("42");
let passwd = String::from("pass");
let sql = prepare!("SELECT name FROM users WHERE id=") + &id + prepare!(" AND passwd=") + &passwd;
assert_eq!(concatsql::actual_sql(&sql), "SELECT name FROM users WHERE id='42' AND passwd='pass'");
for (i, row) in conn.rows(&sql).unwrap().iter().enumerate() {
    assert_eq!(row.get("name").unwrap(), "Alice");
}
```

### Illegal value

```rust
use concatsql::prepare;
let conn = concatsql::sqlite::open(":memory:").unwrap();
let id     = String::from("42");
let passwd = String::from("' or 1=1; --");
let sql = prepare!("SELECT name FROM users WHERE id=") + &id + prepare!(" AND passwd=") + &passwd;
assert_eq!(concatsql::actual_sql(&sql), "SELECT name FROM users WHERE id='42' AND passwd=''' or 1=1; --'");
for (i, row) in conn.rows(&sql).unwrap().iter().enumerate() {
    unreachable!();
}
```

### If you did not use the concatsql::prepare macro

cannot compile

```rust
let conn = concatsql::sqlite::open(":memory:").unwrap();
let id     = String::from("42");
let passwd = String::from("' or 1=1; --");
let sql = "SELECT name FROM users WHERE id=" + &id + " AND passwd='" + &passwd + "';";
conn.execute(&sql).unwrap();  // error
```

### concatsql::prepare!(\<String\>)

cannot compile

```rust
use concatsql::prepare;
let conn = concatsql::sqlite::open(":memory:").unwrap();
let age = String::from("50 or 1=1; --");
let sql = prepare!("SELECT name FROM users WHERE age < ") + prepare!(&age);  // error
```

## License

MIT

