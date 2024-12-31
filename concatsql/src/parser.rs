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
            '"' => escaped.push_str("&quot;"),
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            c => escaped.push(c),
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
/// # use concatsql::prep;
/// let name = "Ali";
/// let sql = prep!("SELECT * FROM users WHERE name LIKE ") + ("%".to_owned() + name + "%");
/// assert_eq!(sql.simulate(), "SELECT * FROM users WHERE name LIKE '%Ali%'");
///
/// let name = String::from("%Ali%");
/// let sql = prep!("SELECT * FROM users WHERE name LIKE ") + ("%".to_owned() + &sanitize_like!(name, '$') + "%");
/// assert_eq!(sql.simulate(), "SELECT * FROM users WHERE name LIKE '%$%Ali$%%'");
/// ```
#[macro_export]
macro_rules! sanitize_like {
    ($pattern:tt) => {
        $crate::_sanitize_like($pattern, '\\')
    };
    ($pattern:tt, $escape:tt) => {
        $crate::_sanitize_like($pattern, $escape)
    };
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
        if c == '\'' {
            escaped.push('\'');
        }
        #[cfg(any(feature = "mysql", feature = "postgres"))]
        if c == '\\' {
            escaped.push('\\');
        }
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

    bytes
        .iter()
        .map(|&n| LUT.get(n as usize).unwrap().to_owned())
        .collect::<String>()
}

pub(crate) fn to_binary_literal(bytes: &[u8]) -> String {
    let data = to_hex(bytes);

    if cfg!(feature = "sqlite") || cfg!(feature = "mysql") {
        format!("X'{}'", data)
    } else {
        format!("'\\x{}'", data)
    }
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
    #[cfg(feature = "sqlite")]
    #[cfg(not(all(feature = "sqlite", feature = "mysql", feature = "postgres")))]
    fn escape_string() {
        assert_eq!(super::escape_string("O'Reilly"), "'O''Reilly'");
        assert_eq!(super::escape_string("O\\'Reilly"), "'O\\''Reilly'");
    }

    #[test]
    #[cfg(feature = "mysql")]
    #[cfg(not(all(feature = "sqlite", feature = "mysql", feature = "postgres")))]
    fn escape_string() {
        assert_eq!(super::escape_string("O'Reilly"), "'O''Reilly'");
        assert_eq!(super::escape_string("O\\'Reilly"), "'O\\\\''Reilly'");
    }

    #[test]
    #[cfg(feature = "postgres")]
    #[cfg(not(all(feature = "sqlite", feature = "mysql", feature = "postgres")))]
    fn escape_string() {
        assert_eq!(super::escape_string("O'Reilly"), "'O''Reilly'");
        assert_eq!(super::escape_string("O\\'Reilly"), "'O\\\\''Reilly'");
    }
}
