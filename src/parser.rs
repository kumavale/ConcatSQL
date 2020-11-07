use crate::Result;
use crate::error::{OwsqlError, OwsqlErrorLevel};

/// Convert special characters to HTML entities.
///
/// # Performed translations
///
/// Character | Replacement
/// --------- | -----------
/// &amp;     | &amp;amp;
/// &quot;    | &amp;quot;
/// &#39;     | &amp;#39;
/// &lt;      | &amp;lt;
/// &gt;      | &amp;gt;
///
/// # Examples
///
/// ```
/// let encoded = exowsql::html_special_chars("<a href='test'>Test</a>");
/// assert_eq!(&encoded, "&lt;a href=&#39;test&#39;&gt;Test&lt;/a&gt;");
/// ```
pub fn html_special_chars(input: &str) -> String {
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

/// Sanitizes a string so that it is safe to use within an SQL LIKE statement.  
/// This method uses escape_character to escape all occurrences of '_' and '%'.  
///
/// # Examples
///
/// ```
/// assert_eq!(exowsql::sanitize_like!("%foo_bar"),      "\\%foo\\_bar");
/// assert_eq!(exowsql::sanitize_like!("%foo_bar", '!'), "!%foo!_bar");
/// ```
#[macro_export]
macro_rules! sanitize_like {
    ($pattern:tt) =>             { exowsql::_sanitize_like($pattern, '\\') };
    ($pattern:tt, $escape:tt) => { exowsql::_sanitize_like($pattern, $escape) };
}
#[doc(hidden)]
pub fn _sanitize_like<T: std::string::ToString>(pattern: T, escape_character: char) -> String {
    let mut escaped_str = String::new();
    for ch in pattern.to_string().chars() {
        if ch == '%' || ch == '_' {
            escaped_str.push(escape_character);
        }
        escaped_str.push(ch);
    }
    escaped_str
}

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
    format!("'{}'", escaped)
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
            #[cfg(debug_assertions)]
            OwsqlErrorLevel::Debug    => OwsqlError::Message("error: next_char(): None".to_string()),
        })
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
            #[cfg(debug_assertions)]
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
                #[cfg(debug_assertions)]
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
            #[cfg(debug_assertions)]
            OwsqlErrorLevel::Debug   => OwsqlError::Message("error: consume_char(): None".to_string()),
        })?;
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        Ok(cur_char)
    }
}

// I want to write with const fn
#[doc(hidden)]
pub fn check_valid_literal(s: &'static str) -> Result<()> {
    let err_msg = "invalid literal";
    let mut parser = Parser::new(&s, &OwsqlErrorLevel::Debug);
    while !parser.eof() {
        parser.consume_while(|c| c != '"' && c != '\'')?;
        match parser.next_char() {
            Ok('"')  => if parser.consume_string('"').is_err() {
                return OwsqlError::new(&OwsqlErrorLevel::Debug, err_msg, &s);
            },
            Ok('\'')  => if parser.consume_string('\'').is_err() {
                return OwsqlError::new(&OwsqlErrorLevel::Debug, err_msg, &s);
            },
            _other => (), // Do nothing
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::error::*;

    #[test]
    fn html_special_chars() {
        assert_eq!(
            super::html_special_chars(r#"<script type="text/javascript">alert('1')</script>"#),
            r#"&lt;script type=&quot;text/javascript&quot;&gt;alert(&#39;1&#39;)&lt;/script&gt;"#
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    fn consume_char() {
        let mut p = super::Parser::new("", &OwsqlErrorLevel::Debug);
        assert_eq!(p.consume_char(), Err(OwsqlError::Message("error: consume_char(): None".into())));
    }

    #[test]
    fn escape_string() {
        assert_eq!(super::escape_string("O'Reilly",   |c| c=='\''),            "'O''Reilly'");
        assert_eq!(super::escape_string("O\\'Reilly", |c| c=='\''),            "'O\\''Reilly'");
        assert_eq!(super::escape_string("O'Reilly",   |c| c=='\'' || c=='\\'), "'O''Reilly'");
        assert_eq!(super::escape_string("O\\'Reilly", |c| c=='\'' || c=='\\'), "'O\\\\''Reilly'");
    }
}
