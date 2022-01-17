#[derive(Debug, Clone, PartialEq, Copy)]
pub struct Address {
    pub num: u16,
}

impl Address {
    pub fn new(num: u16) -> Address {
        Address { num }
    }
}

impl From<u16> for Address {
    fn from(num: u16) -> Address {
        Address::new(num)
    }
}
