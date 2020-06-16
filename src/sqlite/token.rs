
pub enum TokenType {
    //EOF,
    String(String),
    Overwrite(String),
    //Identifier(String),
    //Interger(i64),
    //Float(f64),
    //Binary(Vec<u8>),
}

impl TokenType {
    pub fn unwrap(self) -> String {
        match self {
            TokenType::String(s) => s,
            TokenType::Overwrite(s) => s,
        }
    }
}

