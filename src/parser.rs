use crate::Result;
use crate::error::{Error, ErrorLevel};

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
/// let encoded = concatsql::html_special_chars("<a href='test'>Test</a>");
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
///
/// This method uses escape_character to escape all occurrences of '_' and '%'.
///
/// # Examples
///
/// ```
/// # use concatsql::prelude::*;
/// assert_eq!(sanitize_like!("%foo_bar"),      "\\%foo\\_bar");
/// assert_eq!(sanitize_like!("%foo_bar", '!'), "!%foo!_bar");
/// ```
/// ```
/// # use concatsql::prelude::*;
/// let name = "Ali";
/// let sql = prep!("SELECT * FROM users WHERE name LIKE ") + ("%".to_owned() + name + "%");
/// assert_eq!(sql.actual_sql(), "\"SELECT * FROM users WHERE name LIKE ?\", [\"%Ali%\"]");
///
/// let name = String::from("%Ali%");
/// let sql = prep!("SELECT * FROM users WHERE name LIKE ") + ("%".to_owned() + &sanitize_like!(name, '$') + "%");
/// assert_eq!(sql.actual_sql(), "\"SELECT * FROM users WHERE name LIKE ?\", [\"%$%Ali$%%\"]");
/// ```
#[macro_export]
macro_rules! sanitize_like {
    ($pattern:tt) =>             { concatsql::_sanitize_like($pattern, '\\') };
    ($pattern:tt, $escape:tt) => { concatsql::_sanitize_like($pattern, $escape) };
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

pub(crate) fn escape_string(s: &str) -> String {
    let mut escaped = String::new();
    escaped.push('\'');
    for c in s.chars() {
        if c == '\'' { escaped.push('\''); }
        #[cfg(any(feature = "mysql", feature = "postgres"))]
        if c == '\\' { escaped.push('\\'); }
        escaped.push(c);
    }
    escaped.push('\'');
    escaped
}

pub(crate) fn to_hex(bytes: &[u8]) -> String {
    use lazy_static::lazy_static;
    lazy_static! {
        static ref LUT: Vec<String> = (0u8..=255).map(|n| format!("{:02X}", n)).collect();
    }

    bytes.iter().map(|&n| LUT.get(n as usize).unwrap().to_owned()).collect::<String>()
}

pub(crate) fn to_binary_literal(bytes: &[u8]) -> String {
    let data = to_hex(bytes);

    if cfg!(feature = "sqlite") || cfg!(feature = "mysql") {
        format!("X'{}'", data)
    } else {
        format!("'\\x{}'", data)
    }
}

pub struct Parser<'a> {
    input:       &'a str,
    pos:         usize,
    error_level: &'a ErrorLevel,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str, error_level: &'a ErrorLevel) -> Self {
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
            ErrorLevel::AlwaysOk |
            ErrorLevel::Release  => Error::AnyError,
            ErrorLevel::Develop  => Error::Message("error: next_char()".to_string()),
            #[cfg(debug_assertions)]
            ErrorLevel::Debug    => Error::Message("error: next_char(): None".to_string()),
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
            ErrorLevel::AlwaysOk |
            ErrorLevel::Release  => Error::AnyError,
            ErrorLevel::Develop  => Error::Message("endless".to_string()),
            #[cfg(debug_assertions)]
            ErrorLevel::Debug    => Error::Message(format!("endless: {}", s)),
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
        Ok(s)
    }

    pub fn consume_char(&mut self) -> Result<char> {
        let mut iter = self.input[self.pos..].char_indices();
        let (_, cur_char) = iter.next().ok_or_else(|| match self.error_level {
            ErrorLevel::AlwaysOk |
            ErrorLevel::Release => Error::AnyError,
            ErrorLevel::Develop => Error::Message("error: consume_char()".to_string()),
            #[cfg(debug_assertions)]
            ErrorLevel::Debug   => Error::Message("error: consume_char(): None".to_string()),
        })?;
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        Ok(cur_char)
    }

    pub fn visible_len(&self) -> usize {
        use unicode_width::UnicodeWidthStr;
        UnicodeWidthStr::width(&self.input[..self.pos])
    }
}

// I want to write with const fn
#[doc(hidden)]
pub fn check_valid_literal(s: &'static str) -> Result<()> {
    let mut parser = Parser::new(&s, &ErrorLevel::Develop);
    while !parser.eof() {
        parser.consume_while(|c| c != '"' && c != '\'')?;
        match parser.next_char() {
            Ok(c) => if c == '"' || c == '\'' {
                let visible_len = parser.visible_len();
                if parser.consume_string(c).is_err() {
                    #[cfg(debug_assertions)]
                    let err_msg = format!("    {}\n{:<width1$}\x1b[31m{:^<width2$}\x1b[0m",
                        s, "", "^", width1 = visible_len + 4, width2 = parser.visible_len() - visible_len);
                    #[cfg(not(debug_assertions))]
                    let err_msg = format!("    {}\n{:<width1$}\x1b[33m{:^<width2$}\x1b[0m",
                        s, "", "^", width1 = visible_len + 4, width2 = parser.visible_len() - visible_len);
                    return Err(Error::Message(err_msg));
                }
            }
            _other => (), // Do nothing
        }
    }

    Ok(())
}

#[doc(hidden)]
pub fn invalid_literal() -> &'static str {
    #[cfg(debug_assertions)]
    return "\x1b[31merror\x1b[0m: invalid literal\n";
    #[cfg(not(debug_assertions))]
    return "\x1b[33mwarning\x1b[0m: invalid literal\n";
}

#[cfg(test)]
mod tests {

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
        use crate::error::*;
        let mut p = super::Parser::new("", &ErrorLevel::Debug);
        assert_eq!(p.consume_char(), Err(Error::Message("error: consume_char(): None".into())));
    }

    #[test]
    #[cfg(feature = "sqlite")]
    #[cfg(not(all(feature = "sqlite", feature = "mysql", feature = "postgres")))]
    fn escape_string() {
        assert_eq!(super::escape_string("O'Reilly"),   "'O''Reilly'");
        assert_eq!(super::escape_string("O\\'Reilly"), "'O\\''Reilly'");
    }

    #[test]
    #[cfg(feature = "mysql")]
    #[cfg(not(all(feature = "sqlite", feature = "mysql", feature = "postgres")))]
    fn escape_string() {
        assert_eq!(super::escape_string("O'Reilly"),   "'O''Reilly'");
        assert_eq!(super::escape_string("O\\'Reilly"), "'O\\\\''Reilly'");
    }

    #[test]
    #[cfg(feature = "postgres")]
    #[cfg(not(all(feature = "sqlite", feature = "mysql", feature = "postgres")))]
    fn escape_string() {
        assert_eq!(super::escape_string("O'Reilly"),   "'O''Reilly'");
        assert_eq!(super::escape_string("O\\'Reilly"), "'O\\\\''Reilly'");
    }

    #[test]
    fn check_valid_literal() {
        assert!(super::check_valid_literal("foo").is_ok());
        assert!(super::check_valid_literal("id=").is_ok());
        assert!(super::check_valid_literal("''").is_ok());
        assert!(super::check_valid_literal("'\"'").is_ok());
        assert!(super::check_valid_literal("'\"\"'").is_ok());
        assert!(super::check_valid_literal("\"\"").is_ok());
        assert!(super::check_valid_literal("\"'\"").is_ok());
        assert!(super::check_valid_literal("\"''\"").is_ok());
        assert!(super::check_valid_literal("'O''Reilly'").is_ok());
        assert!(super::check_valid_literal("'foo'+'bar'").is_ok());
        assert!(super::check_valid_literal("").is_ok());
        assert!(super::check_valid_literal("'\"'''").is_ok());

        assert!(!super::check_valid_literal("O'Reilly").is_ok());
        assert!(!super::check_valid_literal("'O'Reilly'").is_ok());
        assert!(!super::check_valid_literal("id='").is_ok());
        assert!(!super::check_valid_literal("'").is_ok());
        assert!(!super::check_valid_literal("\"").is_ok());
        assert!(!super::check_valid_literal("'''").is_ok());
        assert!(!super::check_valid_literal("\"\"\"").is_ok());
        assert!(!super::check_valid_literal("' AND ...").is_ok());
        assert!(!super::check_valid_literal("\\'").is_ok());
        assert!(!super::check_valid_literal("\\\"").is_ok());

        #[cfg(debug_assertions)]
        assert_eq!(
            super::check_valid_literal("O'Reilly").unwrap_err().to_string(),
            "    O'Reilly\n     \x1b[31m^^^^^^^\x1b[0m"
        );
        #[cfg(debug_assertions)]
        assert_eq!(
            super::check_valid_literal("passwd='").unwrap_err().to_string(),
            "    passwd='\n           \x1b[31m^\x1b[0m"
        );
        #[cfg(not(debug_assertions))]
        assert_eq!(
            super::check_valid_literal("O'Reilly").unwrap_err().to_string(),
            "    O'Reilly\n     \x1b[33m^^^^^^^\x1b[0m"
        );
        #[cfg(not(debug_assertions))]
        assert_eq!(
            super::check_valid_literal("passwd='").unwrap_err().to_string(),
            "    passwd='\n           \x1b[33m^\x1b[0m"
        );
    }
}
