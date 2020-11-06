use std::ops::Add;

#[derive(Clone, Debug, PartialEq)]
pub struct OwString {
    pub(crate) query: String,
}

impl OwString {
    pub fn new<T: ?Sized + std::string::ToString>(s: &T) -> Self {
        Self {
            query: s.to_string(),
        }
    }

    /// Return the actual SQL statement.
    ///
    /// # Examples
    ///
    /// ```
    /// # let conn = exowsql::sqlite::open(":memory:").unwrap();
    /// assert_eq!(conn.prepare("SELECT").actual_sql(),   "SELECT");
    /// assert_eq!(conn.bind("SELECT").actual_sql(),      "'SELECT'");
    /// //assert_eq!(conn.prepare("O'Reilly").actual_sql(), "O'Reilly");  // panic
    /// assert_eq!(conn.bind("O'Reilly").actual_sql(),    "'O''Reilly'");
    /// ```
    #[inline]
    pub fn actual_sql(&self) -> &str {
        &self.query
    }
}

impl Add for OwString {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            query: self.query + &other.query,
        }
    }
}

impl<'a> Add<&'a OwString> for OwString {
    type Output = OwString;

    fn add(self, other: &'a OwString) -> OwString {
        OwString {
            query: self.query + &other.query.clone(),
        }
    }
}

