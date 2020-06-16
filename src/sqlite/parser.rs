use super::connection::Connection;
use super::define::*;

macro_rules! overwrite_new {
    () => {
        format!("[{}]",
            thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .collect::<String>())
    };
    ($max: expr) => {
        format!("[{}]",
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
        let tokens = tokenize(&stmt);

        for token in tokens {
            if is_keyword(&token) {
                return Err("invalid syntax".to_string());
            }

            if token == self.ow_or {
                query.push_str(OR);
            } else if token == self.ow_select {
                query.push_str(SELECT);
            } else if token == self.ow_from {
                query.push_str(FROM);
            } else if token == self.ow_where {
                query.push_str(WHERE);
            } else {
                query.push_str(token);
            }

            query.push(' ');
        }

        Ok(query.as_bytes().to_vec())
    }
}

fn tokenize(stmt: &str) -> Vec<&str> {
    stmt.split_whitespace().collect::<_>()
}
