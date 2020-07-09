
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwrap() {
        assert_eq!(TokenType::None.unwrap(),                              "");
        assert_eq!(TokenType::String(String::from("s1")).unwrap(),       "s1");
        assert_eq!(TokenType::Overwrite(String::from("s2")).unwrap(),    "s2");
        assert_eq!(TokenType::ErrOverwrite(String::from("s3")).unwrap(), "s3");
    }

    #[test]
    fn is_none() {
        let tests = [
            (TokenType::None, true),
            (TokenType::String(String::from("s1")), false),
            (TokenType::Overwrite(String::from("s2")), false),
            (TokenType::ErrOverwrite(String::from("s3")), false),
        ];

        for (token, expect) in tests.iter() {
            assert_eq!(token.is_none(), *expect);
        }
    }
}

