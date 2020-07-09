
#[derive(Default)]
pub struct SerialNumber(usize);

impl SerialNumber {
    pub fn get(&mut self) -> usize {
        self.0 += 1;
        self.0 - 1
    }
}

