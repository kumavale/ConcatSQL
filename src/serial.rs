
#[derive(Default)]
pub struct SerialNumber(usize);

impl SerialNumber {
    pub fn get(&mut self) -> usize {
        let ret_val = self.0;
        self.0 += 1;
        ret_val
    }
}

