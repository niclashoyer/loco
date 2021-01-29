use crate::{direction, speed};
use loco_core::{
	drive::{Direction, Speed},
	mov,
};

#[derive(Debug)]
pub enum Message {
	Unknown,
	Drive(Direction, Speed),
}

#[derive(Debug)]
pub struct Address {
	num: u16,
}

impl Address {
	pub fn from_bytes(bytes: &[u8]) -> Self {
		let num = if bytes[0] & 0xC0 == 0xC0 && bytes[0] & 0x3F != 0x3F {
			u16::from_le_bytes([bytes[0] & 0x3F, bytes[1]])
		} else {
			bytes[0] as u16
		};
		Address { num }
	}

	pub fn to_buf(&self, buf: &mut [u8]) -> usize {
		if self.num > 127 {
			mov!(buf[0..=1] <= &self.num.to_le_bytes());
			buf[0] |= 0xC0;
			2
		} else {
			buf[0] = self.num as u8;
			1
		}
	}

	pub fn len(&self) -> usize {
		if self.num > 127 {
			2
		} else {
			1
		}
	}
}

impl Message {
	pub fn from_bytes(bytes: &[u8]) -> Self {
		use Message::*;
		let addr = Address::from_bytes(bytes);
		println!("{:?} {:#04X?}", addr, bytes);
		let bytes = &bytes[addr.len()..];
		let cmd = (bytes[0] & 0b1110_0000) >> 5;
		match cmd {
			0b010 | 0b011 => Drive(
				direction::from_baseline_byte(bytes[0]),
				speed::from_byte_28_steps(bytes[0]),
			),
			_ => Unknown,
		}
	}
}
