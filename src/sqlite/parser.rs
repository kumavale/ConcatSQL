use super::connection::Connection;

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

            if let Some(original) = self.overwrite.get_reverse(&token) {
                query.push_str(original);
            } else {
                query.push_str(&token);
            }

            query.push(' ');
        }

        Ok(query.as_bytes().to_vec())
    }

    fn tokenize(&self, stmt: &str) -> Vec<String> {
        let mut parser = Parser::new(&stmt);
        let mut tokens = Vec::new();

        while !parser.eof() {
            let _ = parser.skip_whitespace();

            if let Ok(string) = parser.consume_string() {
                tokens.push(string);
            } else if parser.next_char_is(';') {
                tokens.push(";".to_string());
                let _ = parser.consume_char();
            } else {
                break;
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

    fn next_char_is(&self, c: char) -> bool {
        self.next_char() == Ok(c)
    }

    fn skip_whitespace(&mut self) -> Result<(), ()> {
        self.consume_while(char::is_whitespace).and(Ok(()))
    }

    fn consume_string(&mut self) -> Result<String, ()> {
        self.consume_while(char::is_alphanumeric) // TODO
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

