use crate::{address::DccAddress, direction::DccDirection, speed::DccSpeed};
use loco_core::{
	address::Address,
	drive::{Direction, Speed},
};
use log::trace;

#[derive(Debug)]
pub enum Message {
	Unknown(Address),
	Drive(Address, Direction, Speed),
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
				addr,
				Direction::from_baseline_byte(bytes[0]),
				Speed::from_byte_28_steps(bytes[0]),
			),
			_ => Unknown(addr),
		}
	}

	pub fn to_buf(&self, buf: &mut [u8]) -> usize {
		use Message::*;
		let add_xor = |buf: &mut [u8], len: usize| -> usize {
			let x = buf[0..len - 1].iter().fold(0, |acc, x| acc ^ x);
			buf[len - 1] = x;
			len
		};
		match self {
			Drive(addr, dir, speed) => {
				// FIXME: does not work for 126 steps
				let n = addr.to_buf(buf);
				buf[n] = 0b0100_0000 | dir.to_baseline_byte() | speed.to_byte();
				add_xor(buf, n + 2)
			}
			_ => unimplemented!(),
		}
	}
}
