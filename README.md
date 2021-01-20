# ConcatSQL

[![Actions Status](https://github.com/kumavale/ConcatSQL/workflows/CI/badge.svg)](https://github.com/kumavale/ConcatSQL/actions)
[![Crates.io](https://img.shields.io/crates/v/concatsql.svg)](https://crates.io/crates/concatsql)
[![Documentation](https://docs.rs/concatsql/badge.svg)](https://docs.rs/concatsql/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat)](LICENSE)
  

ConcatSQL(`concatsql`) is a secure SQL database library.  
You can use string concatenation to prevent SQL injection.  

**[Documentation](https://docs.rs/concatsql/)**  

Supported databases:
- [PostgreSQL](https://www.postgresql.org/)
- [MySQL](https://www.mysql.com/)
- [SQLite](https://sqlite.com/)

You can configure the database backend in `Cargo.toml`:

```toml
[dependencies]
concatsql = { version = "<version>", features = ["<postgres|mysql|sqlite>"] }
```

## Examples

### Normal value

```rust
let id     = String::from("42");    // User supplied input
let passwd = String::from("pass");  // User supplied input

let sql = prep("SELECT name FROM users WHERE id=") + &id + prep(" AND passwd=") + &passwd;
assert_eq!(sql.simulate(), "SELECT name FROM users WHERE id='42' AND passwd='pass'");

for row in conn.rows(&sql).unwrap() {
    assert_eq!(row.get(0).unwrap(),      "Alice");
    assert_eq!(row.get("name").unwrap(), "Alice");
}
```

### Illegal value

```rust
let id     = String::from("42");             // User supplied input
let passwd = String::from("'' or 1=1; --");  // User supplied input

let sql = prep("SELECT name FROM users WHERE id=") + &id + prep(" AND passwd=") + &passwd;
assert_eq!(sql.simulate(), "SELECT name FROM users WHERE id='42' AND passwd=''''' or 1=1; --'");

for row in conn.rows(&sql).unwrap() {
    unreachable!();
}
```

### If you did not use the `prep` function

Cannot compile ... secure!

```rust
let id     = String::from("42");
let passwd = String::from("' or 1=1; --");
let sql = "SELECT name FROM users WHERE id=".to_string() + &id + " AND passwd='" + &passwd + "';";
conn.execute(&sql).unwrap();  // error
```

### When using `prep(<String>)`

Cannot compile ... secure!

```rust
let age = String::from("50 or 1=1; --");
let sql = prep("SELECT name FROM users WHERE age < ") + prep(&age);  // error
```

## Why can this library prevent SQL injection?

This is because it is achieved using [Operator Overloading](https://doc.rust-lang.org/stable/rust-by-example/trait/ops.html) rather than simple string concatenation.  
The `prep` function returns the library's own type(`WrapString`).  
For example, if you combine this `WrapString` type with a `String` type, the escaped `String` type will be combined and a new `WrapString` will be returned.  

```rust
struct WrapString<'a> {
    query:  Vec<Option<Cow<'a, str>>>,
    params: Vec<Value>,
}

let foobar42: WrapString = prep("foo") + String::from("bar") + 42;

foobar42 {
    query:  [Some("foo"), None, None],
    params: [Value::Text("bar"), Value::I32(42)],
}

ffi::sqlite3_prepare_v2(..., "foo??", ...);
ffi::sqlite3_bind_text(..., "bar", ...);
ffi::sqlite3_bind_int(..., 42);
```

## Is it impossible to implement in other languages?

It seems that it can be implemented in other languages as long as it supports operator overloading.  
However, if the developer writes the following, the input from the attacker will not be escaped correctly and the attack will be successful.  

```python
prep("SELECT * FROM users WHERE id=" + id + " AND PASSWORD=" + password)
```

That is, it can be implemented in any language that can distinguish between hard-coding(`&'static str`) and user input(`String`) at compile time.  

## License

MIT

