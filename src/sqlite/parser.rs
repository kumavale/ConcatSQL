use crate::Result;
use crate::error::OwsqlError;
use super::connection::Connection;
use super::token::TokenType;

macro_rules! overwrite_new {
    ($serial: expr) => {
        format!("OWSQL{}{}",
            thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .collect::<String>(),
            $serial.to_string())
    };
    ($serial: expr, $max: expr) => {
        format!("OWSQL{}{}",
            thread_rng()
            .sample_iter(&Alphanumeric)
            .take( if 32 < $max {
                thread_rng().gen_range(32, $max)
            } else {
                32
            })
            .collect::<String>(),
            $serial.to_string())
    };
}

pub(crate) fn escape_for_allowlist(value: &str) -> String {
    debug_assert!({
        let value = format!("'{}'", &value);
        let mut parser = Parser::new(&value);
        parser.consume_string('\'').is_ok()
    });
    let value = format!("'{}'", value);
    let mut parser = Parser::new(&value);
    parser.consume_string('\'').unwrap_or_default()
}

impl Connection {
    pub(crate) fn check_valid_literal(&self, s: &str) -> Result<()> {
        let err_msg = "invalid literal";
        let mut parser = Parser::new(&s);
        while !parser.eof() {
            parser.consume_while(|c| c != '"' && c != '\'').ok();
            match parser.next_char() {
                Ok('"')  => if parser.consume_string('"').is_err() {
                    return Err(self.err(err_msg));
                },
                Ok('\'')  => if parser.consume_string('\'').is_err() {
                    return Err(self.err(err_msg));
                },
                _ => (),
            }
        }

        Ok(())
    }

    pub(crate) fn convert_to_valid_syntax(&self, stmt: &str) -> Result<String> {
        let mut query = String::new();
        let tokens = self.tokenize(&stmt).or_else(|e| Err(self.err(&e.to_string())))?;

        for token in tokens {
            let token = token.unwrap();

            if let Some(e) = self.error_msg.get_reverse(&token) {
                return Err(e.clone());
            } else if let Some(original) = self.overwrite.get_reverse(&token) {
                query.push_str(original);
            } else {
                query.push_str(&token);
            }

            query.push(' ');
        }

        Ok(query)
    }

    fn tokenize(&self, stmt: &str) -> Result<Vec<TokenType>> {
        let mut parser = Parser::new(&stmt);
        let mut tokens = Vec::new();

        while !parser.eof() {
            parser.skip_whitespace().ok();

            match parser.next_char() {
                Ok('"')  => tokens.push(TokenType::String( parser.consume_string('"')?  )),
                Ok('\'') => tokens.push(TokenType::String( parser.consume_string('\'')? )),
                Ok(_other) => {
                    let string = parser.consume_except_whitespace()?;
                    if self.overwrite.contain_reverse(&string) || self.error_msg.contain_reverse(&string) {
                        tokens.push(TokenType::Overwrite(string));
                    } else {
                        let mut string = single_quotaion_escape(&string)?;
                        let mut overwrite = String::new();
                        'untilow: while !parser.eof() {
                            let whitespace = parser.consume_whitespace().unwrap_or_default();
                            while let Ok(s) = parser.consume_except_whitespace() {
                                if self.overwrite.contain_reverse(&s) || self.error_msg.contain_reverse(&s) {
                                    overwrite = s;
                                    break 'untilow;
                                } else {
                                    string.push_str(&whitespace);
                                    string.push_str(&single_quotaion_escape(&s)?);
                                }
                            }
                        }
                        tokens.push(TokenType::String(format!("'{}'", string)));
                        if !overwrite.is_empty() {
                            tokens.push(TokenType::Overwrite(overwrite));
                        }
                    }
                },
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

    fn next_char(&self) -> Result<char> {
        self.input[self.pos..].chars().next().ok_or_else(|| OwsqlError::new("next_char: None"))
    }

    fn skip_whitespace(&mut self) -> Result<()> {
        self.consume_while(char::is_whitespace).and(Ok(()))
    }

    fn consume_whitespace(&mut self) -> Result<String> {
        self.consume_while(char::is_whitespace)
    }

    fn consume_except_whitespace(&mut self) -> Result<String> {
        let mut s = String::new();
        while !self.eof() {
            let c = self.next_char()?;
            if c.is_whitespace() {
                break;
            }
            s.push(self.consume_char()?);
        }
        if s.is_empty() {
            Err(OwsqlError::new("consume_except_whitespace: empty"))
        } else {
            Ok(s)
        }
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

    fn consume_while<F>(&mut self, f: F) -> Result<String>
        where
            F: Fn(char) -> bool,
    {
        let mut s = String::new();
        while !self.eof() && f(self.next_char()?) {
            s.push(self.consume_char()?);
        }
        if s.is_empty() {
            Err(OwsqlError::new("consume_while: empty"))
        } else {
            Ok(s)
        }
    }

    fn consume_char(&mut self) -> Result<char> {
        let mut iter = self.input[self.pos..].char_indices();
        let (_, cur_char) = iter.next().ok_or_else(|| OwsqlError::new("consume_char: None"))?;
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        Ok(cur_char)
    }
}

fn single_quotaion_escape(s: &str) -> Result<String> {
    let mut escaped = String::new();
    for c in s.chars() {
        if c == '\'' {
            escaped.push('\'');
        }
        escaped.push(c);
    }
    debug_assert!(!escaped.is_empty());
    Ok(escaped)
}


#[cfg(test)]
mod tests {
    use crate::error::*;

    #[test]
    fn check_valid_literals() {
        let conn = crate::sqlite::open(":memory:").unwrap();
        assert_eq!(conn.check_valid_literal("O'Reilly"),   Err(OwsqlError::Message("invalid literal".to_string())));
        assert_eq!(conn.check_valid_literal("O\"Reilly"),  Err(OwsqlError::Message("invalid literal".to_string())));
        assert_eq!(conn.check_valid_literal("'O'Reilly'"), Err(OwsqlError::Message("invalid literal".to_string())));
        assert_eq!(conn.check_valid_literal("'O\"Reilly'"),    Ok(()));
        assert_eq!(conn.check_valid_literal("'O''Reilly'"),    Ok(()));
        assert_eq!(conn.check_valid_literal("\"O'Reilly\""),   Ok(()));
        assert_eq!(conn.check_valid_literal("'Alice', 'Bob'"), Ok(()));
    }
}
