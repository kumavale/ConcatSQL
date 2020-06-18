use crate::{OwsqlError, Result};
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

pub(crate) fn check_valid_literal(s: &str) -> Result<()> {
    let err_msg = "invalid literal";
    let mut parser = Parser::new(&s);
    while !parser.eof() {
        if parser.consume_while(|c| c != '"' && c != '\'').is_err() {
            return Err(OwsqlError::Message(err_msg.to_string()));
        }
        match parser.next_char() {
            Ok('"')  => {
                if parser.consume_string('"').is_err() {
                    return Err(OwsqlError::Message(err_msg.to_string()));
                }
            },
            Ok('\'')  => {
                if parser.consume_string('\'').is_err() {
                    return Err(OwsqlError::Message(err_msg.to_string()));
                }
            },
            _ => (),
        }
    }
    Ok(())
}

impl Connection {
    pub(crate) fn convert_to_valid_syntax(&self, stmt: &str) -> Result<Vec<u8>> {
        let mut query = String::new();
        let tokens = self.tokenize(&stmt)?;

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

    fn tokenize(&self, stmt: &str) -> Result<Vec<TokenType>> {
        let mut parser = Parser::new(&stmt);
        let mut tokens = Vec::new();

        while !parser.eof() {
            let _ = parser.skip_whitespace();

            match parser.next_char() {
                Ok('"')  => tokens.push(TokenType::String( parser.consume_string('"')?  )),
                Ok('\'') => tokens.push(TokenType::String( parser.consume_string('\'')? )),
                Ok(_) => {
                    if let Ok(string) = parser.consume_except_whitespace_with_escape() {
                        if self.overwrite.contain_reverse(&string) {
                            tokens.push(TokenType::Overwrite(string));
                        } else {
                            let mut string = format!("'{}", string);
                            let mut overwrite = String::new();
                            'untilow: while !parser.eof() {
                                let whitespace = parser.consume_whitespace().unwrap_or_default();
                                while let Ok(s) = parser.consume_except_whitespace_with_escape() {
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

        Ok(tokens)
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

    fn consume_except_whitespace_with_escape(&mut self) -> Result<String, ()> {
        let mut s = String::new();
        while !self.eof() {
            let c = self.next_char()?;
            if c.is_whitespace() {
                break;
            } else if c == '\'' {
                s.push('\'');
            }
            s.push(self.consume_char()?);
        }
        if s.is_empty() { Err(()) } else { Ok(s) }
    }

    fn consume_string(&mut self, quote: char) -> Result<String> {
        let mut s = quote.to_string();
        self.consume_char()?;

        while !self.eof() {
            if self.next_char()? == quote {
                s.push(self.consume_char()?);
                if self.eof() || self.next_char()? != quote {
                    return Ok(s);
                }
            }
            s.push(self.consume_char()?);
        }

        Err(OwsqlError::Message("endless".to_string()))
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

