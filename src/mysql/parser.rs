use crate::Result;
use crate::token::TokenType;
use crate::parser::*;
use super::connection::MysqlConnection;

impl MysqlConnection {
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
            match token {
                TokenType::ErrOverwrite(e) =>
                    return Err(self.error_msg.borrow().get_reverse(&e).unwrap().clone()),
                TokenType::Overwrite(original) =>
                    query.push_str(self.overwrite.borrow().get_reverse(&original).unwrap()),
                other => query.push_str(&other.unwrap()),
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
                if self.overwrite.borrow().contain_reverse(&string) {
                    tokens.push(TokenType::Overwrite(string));
                } else if self.error_msg.borrow().contain_reverse(&string) {
                    tokens.push(TokenType::ErrOverwrite(string));
                } else {
                    let mut overwrite = TokenType::None;
                    'untilow: while !parser.eof() {
                        let whitespace = parser.consume_whitespace().unwrap_or_default();
                        while let Ok(s) = parser.consume_except_whitespace() {
                            if self.overwrite.borrow().contain_reverse(&s) {
                                overwrite = TokenType::Overwrite(s);
                                break 'untilow;
                            } else if self.error_msg.borrow().contain_reverse(&s) {
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
}

