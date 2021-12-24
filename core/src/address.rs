#[derive(Debug, Clone, PartialEq)]
pub struct Address {
	pub num: u16,
}

impl Address {
	pub fn new(num: u16) -> Address {
		Address { num }
	}
}
