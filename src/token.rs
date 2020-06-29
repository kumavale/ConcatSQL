
#[derive(Debug, PartialEq)]
pub enum TokenType {
    None,
    String(String),
    Overwrite(String),
    ErrOverwrite(String),
}

impl TokenType {
    pub fn unwrap(self) -> String {
        match self {
            TokenType::None => String::new(),
            TokenType::String(s) => s,
            TokenType::Overwrite(s) => s,
            TokenType::ErrOverwrite(s) => s,
        }
    }

    pub fn is_none(&self) -> bool {
        self == &TokenType::None
    }
}

