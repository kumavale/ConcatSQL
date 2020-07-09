use crate::Result;
use crate::bidimap::BidiMap;
use crate::connection::Connection;
use crate::error::{OwsqlError, OwsqlErrorLevel};
use crate::token::TokenType;

#[inline]
pub fn escape_for_allowlist(value: &str) -> String {
    let error_level = OwsqlErrorLevel::default();
    let value = format!("'{}'", value);
    let mut parser = Parser::new(&value, &error_level);
    parser.consume_string('\'').unwrap_or_default()
}

#[inline]
pub fn escape_html(input: &str) -> String {
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
pub(crate) fn escape_string<F>(s: &str, is_escape_char: F) -> String
where
    F: Fn(char) -> bool,
{
    let mut escaped = String::new();
    for c in s.chars() {
        if is_escape_char(c) {
            escaped.push(c);
        }
        escaped.push(c);
    }
    debug_assert!(!escaped.is_empty());
    escaped
}

pub struct Parser<'a> {
    input:       &'a str,
    pos:         usize,
    error_level: &'a OwsqlErrorLevel,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str, error_level: &'a OwsqlErrorLevel) -> Self {
        Self {
            input,
            pos: 0,
            error_level,
        }
    }

    pub fn eof(&self) -> bool {
        self.input.len() <= self.pos
    }

    pub fn next_char(&self) -> Result<char> {
        self.input[self.pos..].chars().next().ok_or_else(|| match self.error_level {
            OwsqlErrorLevel::AlwaysOk |
            OwsqlErrorLevel::Release  => OwsqlError::AnyError,
            OwsqlErrorLevel::Develop  => OwsqlError::Message("error: next_char()".to_string()),
            OwsqlErrorLevel::Debug    => OwsqlError::Message("error: next_char(): None".to_string()),
        })
    }

    pub fn skip_whitespace(&mut self) -> Result<()> {
        self.consume_while(char::is_whitespace).and(Ok(()))
    }

    pub fn consume_whitespace(&mut self) -> Result<String> {
        self.consume_while(char::is_whitespace)
    }

    pub fn consume_except_whitespace(&mut self) -> Result<String> {
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
                OwsqlErrorLevel::Develop  => OwsqlError::Message("error: consume_except_whitespace()".to_string()),
                OwsqlErrorLevel::Debug    => OwsqlError::Message("error: consume_except_whitespace(): empty".to_string()),
            })
        } else {
            Ok(s)
        }
    }

    pub fn consume_string(&mut self, quote: char) -> Result<String> {
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
            OwsqlErrorLevel::Develop  => OwsqlError::Message("endless".to_string()),
            OwsqlErrorLevel::Debug    => OwsqlError::Message(format!("endless: {}", s)),
        })
    }

    pub fn consume_while<F>(&mut self, f: F) -> Result<String>
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
                OwsqlErrorLevel::Develop  => OwsqlError::Message("error: consume_while()".to_string()),
                OwsqlErrorLevel::Debug    => OwsqlError::Message("error: consume_while(): empty".to_string()),
            })
        } else {
            Ok(s)
        }
    }

    pub fn consume_char(&mut self) -> Result<char> {
        let mut iter = self.input[self.pos..].char_indices();
        let (_, cur_char) = iter.next().ok_or_else(|| match self.error_level {
            OwsqlErrorLevel::AlwaysOk |
            OwsqlErrorLevel::Release => OwsqlError::AnyError,
            OwsqlErrorLevel::Develop => OwsqlError::Message("error: consume_char()".to_string()),
            OwsqlErrorLevel::Debug   => OwsqlError::Message("error: consume_char(): None".to_string()),
        })?;
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        Ok(cur_char)
    }
}

#[inline]
fn check_valid_literal(s: &str, error_level: &OwsqlErrorLevel) -> Result<()> {
    let err_msg = "invalid literal";
    let mut parser = Parser::new(&s, &error_level);
    while !parser.eof() {
        parser.consume_while(|c| c != '"' && c != '\'').ok();
        match parser.next_char() {
            Ok('"')  => if parser.consume_string('"').is_err() {
                return OwsqlError::new(error_level, err_msg, &s);
            },
            Ok('\'')  => if parser.consume_string('\'').is_err() {
                return OwsqlError::new(error_level, err_msg, &s);
            },
            _other => (), // Do nothing
        }
    }

    Ok(())
}

#[inline]
fn convert_to_valid_syntax(
    stmt:           &str,
    conn_overwrite: &BidiMap<String, String>,
    conn_error_msg: &BidiMap<OwsqlError, String>,
    error_level:    &OwsqlErrorLevel,
) -> Result<String> {

    let mut query = String::new();
    let tokens = tokenize(stmt, conn_overwrite, conn_error_msg, error_level)?;

    for token in tokens {
        match token {
            TokenType::ErrOverwrite(e) =>
                return Err(conn_error_msg.get_reverse(&e).unwrap().clone()),
            TokenType::Overwrite(original) =>
                query.push_str(conn_overwrite.get_reverse(&original).unwrap()),
            other => query.push_str(&other.unwrap()),
        }

        query.push(' ');
    }

    Ok(query)
}

#[inline]
fn tokenize(
    stmt:           &str,
    conn_overwrite: &BidiMap<String, String>,
    conn_error_msg: &BidiMap<OwsqlError, String>,
    error_level:    &OwsqlErrorLevel,
) -> Result<Vec<TokenType>> {

    let mut parser = Parser::new(&stmt, &error_level);
    let mut tokens = Vec::new();

    while !parser.eof() {
        parser.skip_whitespace().ok();

        if parser.next_char().is_ok() {
            let mut string = parser.consume_except_whitespace()?;
            if conn_overwrite.contain_reverse(&string) {
                tokens.push(TokenType::Overwrite(string));
            } else if conn_error_msg.contain_reverse(&string) {
                tokens.push(TokenType::ErrOverwrite(string));
            } else {
                let mut overwrite = TokenType::None;
                'untilow: while !parser.eof() {
                    let whitespace = parser.consume_whitespace().unwrap_or_default();
                    while let Ok(s) = parser.consume_except_whitespace() {
                        if conn_overwrite.contain_reverse(&s) {
                            overwrite = TokenType::Overwrite(s);
                            break 'untilow;
                        } else if conn_error_msg.contain_reverse(&s) {
                            overwrite = TokenType::ErrOverwrite(s);
                            break 'untilow;
                        } else {
                            string.push_str(&whitespace);
                            string.push_str(&s);
                        }
                    }
                }
                tokens.push(TokenType::String(format!("'{}'", escape_html(&string))));
                if !overwrite.is_none() {
                    tokens.push(overwrite);
                }
            }
        }
    }

    Ok(tokens)
}

impl Connection {
    #[inline]
    pub(crate) fn check_valid_literal(&self, s: &str) -> Result<()> {
        check_valid_literal(&s, &self.error_level)
    }

    #[inline]
    pub(crate) fn convert_to_valid_syntax(&self, stmt: &str) -> Result<String> {
        convert_to_valid_syntax(&stmt, &self.overwrite.borrow(), &self.error_msg.borrow(), &self.error_level)
    }
}


#[cfg(test)]
mod tests {
    use crate::error::*;

    #[test]
    #[cfg(features = "sqlite")]
    fn check_valid_literals_sqlite() {
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
    #[cfg(features = "mysql")]
    fn check_valid_literals_mysql() {
        let conn = owsql::mysql::open("mysql://localhost:3306/test").unwrap();
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

    #[test]
    fn escape_string() {
        assert_eq!(super::escape_string("O'Reilly",   |c| c=='\''),            "O''Reilly");
        assert_eq!(super::escape_string("O\\'Reilly", |c| c=='\''),            "O\\''Reilly");
        assert_eq!(super::escape_string("O'Reilly",   |c| c=='\'' || c=='\\'), "O''Reilly");
        assert_eq!(super::escape_string("O\\'Reilly", |c| c=='\'' || c=='\\'), "O\\\\''Reilly");
    }
}
