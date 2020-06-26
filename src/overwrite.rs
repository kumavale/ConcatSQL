
/// Generate new overwrite string.
#[doc(hidden)]
pub(crate) fn overwrite_new(serial: usize, range: (usize, usize)) -> String {
    use rand::{Rng, thread_rng};
    use rand::distributions::Alphanumeric;
    use std::cmp::Ordering;

    format!("OWSQL{}{}",
        thread_rng()
        .sample_iter(Alphanumeric)
        .take( match (range.0).cmp(&range.1) {
            Ordering::Equal   => range.0,
            Ordering::Less    => thread_rng().gen_range(range.0, range.1),
            Ordering::Greater => thread_rng().gen_range(range.1, range.0),
        })
        .collect::<String>(),
        serial.to_string())
}

pub trait IntoInner { fn into_inner(self) -> (usize, usize); }
impl IntoInner for usize                           { fn into_inner(self) -> (usize, usize) { (self, self) } }
impl IntoInner for std::ops::RangeInclusive<usize> { fn into_inner(self) -> (usize, usize) { self.into_inner() } }
impl IntoInner for std::ops::Range<usize> {
    fn into_inner(self) -> (usize, usize) {
        use std::cmp::Ordering;
        match (self.start).cmp(&self.end) {
            Ordering::Equal   => (self.start, self.end),
            Ordering::Less    => (self.start, self.end-1),
            Ordering::Greater => (self.start, self.end+1),
        }
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn into_inner() {
        use super::IntoInner;
        assert_eq!(( 0,  0), (0).into_inner());
        assert_eq!((42, 42), (42).into_inner());
        assert_eq!(( 0, 31), (0..32).into_inner());
        assert_eq!(( 0, 32), (0..=32).into_inner());
        assert_eq!((64, 64), (64..64).into_inner());
        assert_eq!((64, 64), (64..=64).into_inner());
        assert_eq!((64, 33), (64..32).into_inner());
        assert_eq!((64, 32), (64..=32).into_inner());
    }
}

