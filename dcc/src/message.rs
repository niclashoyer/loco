use crate::{address::DccAddress, direction::DccDirection, speed::DccSpeed};
use loco_core::{
	address::Address,
	drive::{Direction, Speed},
};
use log::trace;

#[derive(Debug)]
pub enum Message {
	Unknown,
	Drive(Direction, Speed),
}

impl Message {
	pub fn from_bytes(bytes: &[u8]) -> Self {
		use Message::*;
		let addr = Address::from_bytes(bytes);
		trace!("{:?} {:#04X?}", addr, bytes);
		let bytes = &bytes[addr.len()..];
		let cmd = (bytes[0] & 0b1110_0000) >> 5;
		match cmd {
			0b010 | 0b011 => Drive(
				Direction::from_baseline_byte(bytes[0]),
				Speed::from_byte_28_steps(bytes[0]),
			),
			_ => Unknown,
		}
	}
}
