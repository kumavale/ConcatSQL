# ConcatSQL

[![Actions Status](https://github.com/kumavale/ConcatSQL/workflows/CI/badge.svg)](https://github.com/kumavale/ConcatSQL/actions)
[![Crates.io](https://img.shields.io/crates/v/concatsql.svg)](https://crates.io/crates/concatsql)
[![Documentation](https://docs.rs/concatsql/badge.svg)](https://docs.rs/concatsql/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat)](LICENSE)
  

ConcatSQL(`concatsql`) is a secure SQL database library.  
You can use string concatenation to prevent SQL injection.  

**[Documentation](https://docs.rs/concatsql/)**  

Supported databases:
- PostgreSQL
- MySQL
- SQLite

You can configure the database backend in `Cargo.toml`:

```toml
[dependencies]
concatsql = { version = "<version>", features = ["<postgres|mysql|sqlite>"] }
```

## Examples

### Normal value

```rust
use concatsql::prelude::*;
let conn = concatsql::sqlite::open(":memory:").unwrap();
let id     = String::from("42");
let passwd = String::from("pass");

let sql = prepare!("SELECT name FROM users WHERE id=") + &id + prepare!(" AND passwd=") + &passwd;
assert_eq!(sql.actual_sql(), "SELECT name FROM users WHERE id='42' AND passwd='pass'");

for (i, row) in conn.rows(&sql).unwrap().iter().enumerate() {
    assert_eq!(row.get("name").unwrap(), "Alice");
}
```

### Illegal value

```rust
use concatsql::prelude::*;
let conn = concatsql::sqlite::open(":memory:").unwrap();
let id     = String::from("42");
let passwd = String::from("'' or 1=1; --");

let sql = prepare!("SELECT name FROM users WHERE id=") + &id + prepare!(" AND passwd=") + &passwd;
assert_eq!(sql.actual_sql(), "SELECT name FROM users WHERE id='42' AND passwd=''''' or 1=1; --'");

for (i, row) in conn.rows(&sql).unwrap().iter().enumerate() {
    unreachable!();
}
```

### If you did not use the `prepare` macro

cannot compile ... secure!

```rust
let conn = concatsql::sqlite::open(":memory:").unwrap();
let id     = String::from("42");
let passwd = String::from("' or 1=1; --");
let sql = "SELECT name FROM users WHERE id=" + &id + " AND passwd='" + &passwd + "';";
conn.execute(&sql).unwrap();  // error
```

### When using `prepare!(\<String\>)`

cannot compile ... secure!

```rust
use concatsql::prepare;
let conn = concatsql::sqlite::open(":memory:").unwrap();
let age = String::from("50 or 1=1; --");
let sql = prepare!("SELECT name FROM users WHERE age < ") + prepare!(&age);  // error
```

## License

MIT

