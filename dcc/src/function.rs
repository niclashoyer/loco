use loco_core::functions::Function;

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct FunctionGroupByte {
    data: u8,
}

impl FunctionGroupByte {
    #[inline]
    fn function_position(f: Function) -> u8 {
        use num_traits::ToPrimitive;
        let n = f.to_u8().unwrap();
        match n {
            0 => 4,
            1..=4 => n - 1,
            _ => (n - 5) % 8,
        }
    }

    #[inline]
    pub fn get(&self, f: Function) -> bool {
        let p = Self::function_position(f);
        (self.data >> p) & 0x01 == 0x01
    }

    #[inline]
    pub fn set(&mut self, f: Function, value: bool) {
        let p = Self::function_position(f);
        if value {
            self.data |= 1 << p;
        } else {
            self.data &= !(1 << p);
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.data = 0x00;
    }
}

impl From<u8> for FunctionGroupByte {
    #[inline]
    fn from(data: u8) -> Self {
        Self { data }
    }
}

impl From<FunctionGroupByte> for u8 {
    #[inline]
    fn from(data: FunctionGroupByte) -> u8 {
        data.data
    }
}
