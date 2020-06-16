use super::connection::Connection;

use std::collections::HashSet;

use lazy_static::lazy_static;

// Reserved keywords
type Keyword = &'static str;
pub const OR: Keyword = "OR";

lazy_static! {
    static ref RESERVED_WORDS: HashSet<String> = {
        let mut hs = HashSet::new();
        hs.insert(OR.to_string());
        hs
    };
}

pub fn is_keyword(token: &str) -> bool {
    RESERVED_WORDS.contains(&token.to_ascii_uppercase())
}

impl Connection {
    pub fn or(&self) -> String {
        format!(" {} ", self.or)
    }
}

