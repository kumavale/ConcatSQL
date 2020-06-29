use crate::Result;
use crate::error::{OwsqlError, OwsqlErrorLevel};
use crate::token::TokenType;
use super::connection::SqliteConnection;

#[inline]
pub(crate) fn escape_for_allowlist(value: &str) -> String {
    let error_level = OwsqlErrorLevel::default();
    let value = format!("'{}'", value);
    let mut parser = Parser::new(&value, &error_level);
    parser.consume_string('\'').unwrap_or_default()
}

#[inline]
fn escape_html(input: &str) -> String {
    let mut escaped = String::new();
    for c in input.chars() {
        match c {
            '\'' => escaped.push_str("&#39;"),
            '"'  => escaped.push_str("&quot;"),
            '&'  => escaped.push_str("&amp;"),
            '<'  => escaped.push_str("&lt;"),
            '>'  => escaped.push_str("&gt;"),
             c   => escaped.push(c),
        }
    }
    escaped
}

#[inline]
pub(crate) fn single_quotaion_escape(s: &str) -> String {
    let mut escaped = String::new();
    for c in s.chars() {
        if c == '\'' {
            escaped.push('\'');
        }
        escaped.push(c);
    }
    debug_assert!(!escaped.is_empty());
    escaped
}

impl SqliteConnection {
    #[inline]
    pub(crate) fn check_valid_literal(&self, s: &str) -> Result<()> {
        let err_msg = "invalid literal";
        let mut parser = Parser::new(&s, &self.error_level);
        while !parser.eof() {
            parser.consume_while(|c| c != '"' && c != '\'').ok();
            match parser.next_char() {
                Ok('"')  => if parser.consume_string('"').is_err() {
                    return self.err(err_msg, &s);
                },
                Ok('\'')  => if parser.consume_string('\'').is_err() {
                    return self.err(err_msg, &s);
                },
                _other => (), // Do nothing
            }
        }

        Ok(())
    }

    #[inline]
    pub(crate) fn convert_to_valid_syntax(&self, stmt: &str) -> Result<String> {
        let mut query = String::new();
        let tokens = self.tokenize(&stmt)?;

        for token in tokens {
            let token = token.unwrap();

            if let Some(e) = self.error_msg.borrow().get_reverse(&token) {
                return Err(e.clone());
            } else if let Some(original) = self.overwrite.borrow().get_reverse(&token) {
                query.push_str(original);
            } else {
                query.push_str(&token);
            }

            query.push(' ');
        }

        Ok(query)
    }

    fn tokenize(&self, stmt: &str) -> Result<Vec<TokenType>> {
        let mut parser = Parser::new(&stmt, &self.error_level);
        let mut tokens = Vec::new();

        while !parser.eof() {
            parser.skip_whitespace().ok();

            if parser.next_char().is_ok() {
                let mut string = parser.consume_except_whitespace()?;
                if self.overwrite.borrow().contain_reverse(&string) || self.error_msg.borrow().contain_reverse(&string) {
                    tokens.push(TokenType::Overwrite(string));
                } else {
                    let mut overwrite = String::new();
                    'untilow: while !parser.eof() {
                        let whitespace = parser.consume_whitespace().unwrap_or_default();
                        while let Ok(s) = parser.consume_except_whitespace() {
                            if self.overwrite.borrow().contain_reverse(&s) || self.error_msg.borrow().contain_reverse(&s) {
                                overwrite = s;
                                break 'untilow;
                            } else {
                                string.push_str(&whitespace);
                                string.push_str(&s);
                            }
                        }
                    }
                    tokens.push(TokenType::String(format!("'{}'", escape_html(&string))));
                    if !overwrite.is_empty() {
                        tokens.push(TokenType::Overwrite(overwrite));
                    }
                }
            }
        }

        Ok(tokens)
    }
}

struct Parser<'a> {
    input:       &'a str,
    pos:         usize,
    error_level: &'a OwsqlErrorLevel,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str, error_level: &'a OwsqlErrorLevel) -> Self {
        Self {
            input,
            pos: 0,
            error_level,
        }
    }

    fn eof(&self) -> bool {
        self.input.len() <= self.pos
    }

    fn next_char(&self) -> Result<char> {
        self.input[self.pos..].chars().next().ok_or_else(|| match self.error_level {
            OwsqlErrorLevel::AlwaysOk |
            OwsqlErrorLevel::Release  => OwsqlError::AnyError,
            OwsqlErrorLevel::Develop  => OwsqlError::new("error: next_char()"),
            OwsqlErrorLevel::Debug    => OwsqlError::new("error: next_char(): None"),
        })
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
            Err( match self.error_level {
                OwsqlErrorLevel::AlwaysOk |
                OwsqlErrorLevel::Release  => OwsqlError::AnyError,
                OwsqlErrorLevel::Develop  => OwsqlError::new("error: consume_except_whitespace()"),
                OwsqlErrorLevel::Debug    => OwsqlError::new("error: consume_except_whitespace(): empty"),
            })
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

        Err( match self.error_level {
            OwsqlErrorLevel::AlwaysOk |
            OwsqlErrorLevel::Release  => OwsqlError::AnyError,
            OwsqlErrorLevel::Develop  => OwsqlError::new("endless"),
            OwsqlErrorLevel::Debug    => OwsqlError::new(format!("endless: {}", s)),
        })
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
            Err( match self.error_level {
                OwsqlErrorLevel::AlwaysOk |
                OwsqlErrorLevel::Release  => OwsqlError::AnyError,
                OwsqlErrorLevel::Develop  => OwsqlError::new("error: consume_while()"),
                OwsqlErrorLevel::Debug    => OwsqlError::new("error: consume_while(): empty"),
            })
        } else {
            Ok(s)
        }
    }

    fn consume_char(&mut self) -> Result<char> {
        let mut iter = self.input[self.pos..].char_indices();
        let (_, cur_char) = iter.next().ok_or_else(|| match self.error_level {
            OwsqlErrorLevel::AlwaysOk |
            OwsqlErrorLevel::Release => OwsqlError::AnyError,
            OwsqlErrorLevel::Develop => OwsqlError::new("error: consume_char()"),
            OwsqlErrorLevel::Debug   => OwsqlError::new("error: consume_char(): None"),
        })?;
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        Ok(cur_char)
    }
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

    #[test]
    fn escape_html() {
        assert_eq!(
            super::escape_html(r#"<script type="text/javascript">alert('1')</script>"#),
            r#"&lt;script type=&quot;text/javascript&quot;&gt;alert(&#39;1&#39;)&lt;/script&gt;"#
        );
    }

    #[test]
    fn consume_char() {
        let mut p = super::Parser::new("", &OwsqlErrorLevel::Debug);
        assert_eq!(p.consume_char(), Err(OwsqlError::Message("error: consume_char(): None".into())));
    }
}
