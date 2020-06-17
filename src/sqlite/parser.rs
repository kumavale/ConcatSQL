use super::connection::Connection;
use super::token::TokenType;

macro_rules! overwrite_new {
    () => {
        format!("OWSQL{}",
            thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .collect::<String>())
    };
    ($max: expr) => {
        format!("OWSQL{}",
            thread_rng()
            .sample_iter(&Alphanumeric)
            .take( if 32 < $max {
                thread_rng().gen_range(32, $max)
            } else {
                32
            })
            .collect::<String>())
    };
}

impl Connection {
    pub fn convert_to_valid_syntax(&self, stmt: &str) -> Result<Vec<u8>, String> {
        let mut query = String::new();
        let tokens = self.tokenize(&stmt);

        for token in tokens {
            let token = token.unwrap();

            if let Some(original) = self.overwrite.get_reverse(&token) {
                query.push_str(original);
            } else {
                query.push_str(&token);
            }

            query.push(' ');
        }

        Ok(query.as_bytes().to_vec())
    }

    fn tokenize(&self, stmt: &str) -> Vec<TokenType> {
        let mut parser = Parser::new(&stmt);
        let mut tokens = Vec::new();

        while !parser.eof() {
            let _ = parser.skip_whitespace();

            match parser.next_char() {
                Ok('"') => {
                    let mut string = String::from("\"");
                    let _ = parser.consume_char();
                    if let Ok(content) = parser.consume_string() {
                        string.push_str(&content);
                        string.push('"');
                        let _ = parser.consume_char();
                    }
                    tokens.push(TokenType::String(string));
                },
                Ok(_) => {
                    if let Ok(string) = parser.consume_except_whitespace() {
                        if self.overwrite.contain_reverse(&string) {
                            tokens.push(TokenType::Overwrite(string));
                        } else {
                            let mut string = format!("'{}", string);
                            let mut overwrite = String::new();
                            'untilow: while !parser.eof() {
                                let whitespace = parser.consume_whitespace().unwrap_or_default();
                                while let Ok(s) = parser.consume_except_whitespace() {
                                    if self.overwrite.contain_reverse(&s) {
                                        overwrite = s;
                                        break 'untilow;
                                    } else {
                                        string.push_str(&whitespace);
                                        string.push_str(&s);
                                    }
                                }
                            }
                            string.push('\'');
                            tokens.push(TokenType::String(string));
                            if !overwrite.is_empty() {
                                tokens.push(TokenType::Overwrite(overwrite));
                            }
                        }
                    }
                }
                _ => break,
            }
        }

        tokens
    }
}

struct Parser<'a> {
    input: &'a str,
    pos:   usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
        }
    }

    fn eof(&self) -> bool {
        self.input.len() <= self.pos
    }

    fn next_char(&self) -> Result<char, ()> {
        self.input[self.pos..].chars().next().ok_or(())
    }

    fn skip_whitespace(&mut self) -> Result<(), ()> {
        self.consume_while(char::is_whitespace).and(Ok(()))
    }

    fn consume_whitespace(&mut self) -> Result<String, ()> {
        self.consume_while(char::is_whitespace)
    }

    fn consume_except_whitespace(&mut self) -> Result<String, ()> {
        self.consume_while(|c| !c.is_whitespace())
    }

    fn consume_string(&mut self) -> Result<String, ()> {
        // TODO
        //self.consume_while(|_| self.input[self.pos..].starts_with("\\\""))
        self.consume_while(|c| c != '"')
    }

    fn consume_while<F>(&mut self, f: F) -> Result<String, ()>
        where
            F: Fn(char) -> bool,
    {
        let mut s = String::new();
        while !self.eof() && f(self.next_char()?) {
            s.push(self.consume_char()?);
        }
        if s.is_empty() {
            Err(())
        } else {
            Ok(s)
        }
    }

    fn consume_char(&mut self) -> Result<char, ()> {
        let mut iter = self.input[self.pos..].char_indices();
        let (_, cur_char) = iter.next().ok_or(())?;
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        Ok(cur_char)
    }
}

