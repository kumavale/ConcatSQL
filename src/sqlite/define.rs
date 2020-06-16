use super::connection::Connection;

use std::collections::HashSet;

use lazy_static::lazy_static;

// Reserved keywords
type Keyword = &'static str;
pub const OR:     Keyword = "OR";
pub const SELECT: Keyword = "SELECT";
pub const FROM:   Keyword = "FROM";
pub const WHERE:  Keyword = "WHERE";

lazy_static! {
    static ref RESERVED_WORDS: HashSet<String> = {
        let mut hs = HashSet::new();
        hs.insert(OR.to_string());
        hs.insert(SELECT.to_string());
        hs.insert(FROM.to_string());
        hs.insert(WHERE.to_string());
        hs
    };
}

pub fn is_keyword(token: &str) -> bool {
    RESERVED_WORDS.contains(&token.to_ascii_uppercase())
}

impl Connection {
    pub fn or(&self) -> String {
        format!(" {} ", self.ow_or)
    }

    pub fn select(&self) -> String {
        format!(" {} ", self.ow_select)
    }

    pub fn from(&self) -> String {
        format!(" {} ", self.ow_from)
    }

    pub fn r#where(&self) -> String {
        format!(" {} ", self.ow_where)
    }
}

