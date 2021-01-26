pub mod direction {
	use loco_core::drive::Direction;

	#[inline]
	pub fn from_baseline_byte(byte: u8) -> Direction {
		if byte & (1 << 5) != 0 {
			Direction::Forward
		} else {
			Direction::Backward
		}
	}

	#[inline]
	pub fn to_baseline_byte(dir: &Direction) -> u8 {
		match dir {
			Direction::Forward => 0x20,
			Direction::Backward => 0x00,
		}
	}

	#[inline]
	pub fn from_advanced_byte(byte: u8) -> Direction {
		if byte & (1 << 7) != 0 {
			Direction::Forward
		} else {
			Direction::Backward
		}
	}

	#[inline]
	pub fn to_advanced_byte(dir: &Direction) -> u8 {
		match dir {
			Direction::Forward => 0x80,
			Direction::Backward => 0x00,
		}
	}
}

pub mod speed {
	use loco_core::drive::Speed;

	#[inline]
	pub fn from_byte_14_steps(byte: u8) -> Speed {
		use Speed::*;
		match byte & 0x0F {
			0x0 => Stop,
			0x01 => EmergencyStop,
			s => Steps14(s * 8),
		}
	}

	#[inline]
	pub fn from_byte_28_steps(byte: u8) -> Speed {
		use Speed::*;
		match byte & 0x1F {
			0x00 => Stop,
			0x01 => EmergencyStop,
			s => Steps28(((s << 1) | ((byte >> 4) & 0x01)) * 4),
		}
	}

	#[inline]
	pub fn from_byte_128_steps(byte: u8) -> Speed {
		use Speed::*;
		match byte & 0x7F {
			0x0 => Stop,
			0x01 => EmergencyStop,
			_ => Steps128((byte & 0x7F) * 2),
		}
	}

	#[inline]
	pub fn to_byte(speed: &Speed) -> u8 {
		use Speed::*;
		match speed {
			Stop => 0x00,
			EmergencyStop => 0x01,
			Steps14(s) => (s / 4) & 0x0F,
			Steps28(s) => (((s / 8) & 0x0F) << 1) | (((s / 4) & 0x01) << 4),
			Steps128(s) => (s / 2) & 0x7F,
		}
	}
}

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
