use loco_core::drive::Direction;

pub trait Byte<T> {
	fn from_byte(byte: &u8) -> T;
	fn to_byte(&self) -> u8;
}

impl Byte<Direction> for Direction {
	fn from_byte(val: &u8) -> Self {
		if val & (1 << 5) != 0 {
			Self::Forward
		} else {
			Self::Backward
		}
	}

	fn to_byte(&self) -> u8 {
		match self {
			Direction::Forward => 0x20,
			Direction::Backward => 0x00,
		}
	}
}

use loco_core::functions::Function;

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct FunctionGroupByte {
	data: u8,
}

impl FunctionGroupByte {
	fn function_position(f: Function) -> u8 {
		use num_traits::ToPrimitive;
		let n = f.to_u8().unwrap();
		match n {
			0 => 4,
			1..=4 => n - 1,
			_ => (n - 5) % 8,
		}
	}

	pub fn get(&self, f: Function) -> bool {
		let p = Self::function_position(f);
		(self.data >> p) & 0x01 == 0x01
	}

	pub fn set(&mut self, f: Function, value: bool) {
		let p = Self::function_position(f);
		if value {
			self.data |= 1 << p;
		} else {
			self.data &= !(1 << p);
		}
	}

	pub fn clear(&mut self) {
		self.data = 0x00;
	}
}

impl From<u8> for FunctionGroupByte {
	fn from(data: u8) -> Self {
		Self { data }
	}
}

impl From<FunctionGroupByte> for u8 {
	fn from(data: FunctionGroupByte) -> u8 {
		data.data
	}
}
