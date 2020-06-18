
#[derive(Debug)]
pub enum TokenType {
    String(String),
    Overwrite(String),
}

impl TokenType {
    pub fn unwrap(self) -> String {
        match self {
            TokenType::String(s) => s,
            TokenType::Overwrite(s) => s,
        }
    }
}

