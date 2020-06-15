
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

pub fn convert_to_valid_syntax(stmt: &str) -> Vec<u8> {
    stmt.as_bytes().to_vec()
}

