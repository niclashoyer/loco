#![no_std]
use core::convert::TryFrom;
use embedded_hal as hal;
use nb;

use hal::digital::{InputPin, OutputPin};

pub struct Susi<DATA, CLK, ACK> {
	pin_data: DATA,
	pin_clk: CLK,
	pin_ack: ACK,
	current_byte: usize,
	buf: [u8; 3],
	last_clk: bool,
	bits_read: u8,
}

#[derive(Debug)]
pub enum Error {
	IOError,
}

#[derive(Debug, PartialEq)]
pub enum Direction {
	Forward,
	Backward,
}

impl From<u8> for Direction {
	fn from(val: u8) -> Self {
		if val & (1 << 7) != 0 {
			Self::Forward
		} else {
			Self::Backward
		}
	}
}

#[derive(Debug, PartialEq)]
pub enum Msg {
	Noop,
	TriggerPulse,
	SpeedDiff(i8),
	MotorPower(i8),
	LocomotiveSpeed(Direction, u8),
	ControlSpeed(Direction, u8),
	LocomotiveLoad(u8),
	Unknown,
}

static MASK7: u8 = 0b01111111;

impl From<[u8; 3]> for Msg {
	fn from(bytes: [u8; 3]) -> Self {
		match bytes[0] {
			0 => Msg::Noop,
			33 => Msg::TriggerPulse,
			34 => Msg::SpeedDiff(bytes[1] as i8),
			35 => Msg::MotorPower(bytes[1] as i8),
			36 => Msg::LocomotiveSpeed(bytes[1].into(), bytes[1] & MASK7),
			37 => Msg::ControlSpeed(bytes[1].into(), bytes[1] & MASK7),
			38 => Msg::LocomotiveLoad(bytes[1] & MASK7),
			_ => Msg::Unknown,
		}
	}
}

#[derive(Debug)]
pub enum MsgError {
	Incomplete,
}

impl Msg {
	pub fn len(cmd: u8) -> usize {
		match cmd {
			0x77 | 0x7B | 0x7F => 3,
			_ => 2,
		}
	}
}

impl<DATA, CLK, ACK> Susi<DATA, CLK, ACK>
where
	DATA: InputPin,
	CLK: InputPin,
	ACK: OutputPin,
{
	pub fn new(pin_data: DATA, pin_clk: CLK, pin_ack: ACK) -> Self {
		let last_clk = pin_clk.try_is_high().unwrap_or(false);
		Self {
			pin_data,
			pin_clk,
			pin_ack,
			current_byte: 0,
			buf: [0; 3],
			last_clk,
			bits_read: 0,
		}
	}

	pub fn read(&mut self) -> nb::Result<Msg, Error> {
		// get current clock signal
		let clk = self.pin_clk.try_is_high().map_err(|_| Error::IOError)?;
		// check if we have a falling edge
		if self.last_clk && !clk {
			// read data on falling edge
			let data = if self.pin_data.try_is_high().map_err(|_| Error::IOError)? {
				1
			} else {
				0
			};
			// push bit into buffer
			self.buf[self.current_byte] |= data << self.bits_read;
			self.bits_read += 1;
		}
		// safe clock signal to detect next falling edge
		self.last_clk = clk;
		// full byte read
		if self.bits_read == 7 {
			// TODO: handle 8ms sync timeout
			// prepare to read the next byte
			self.bits_read = 0;
			// check if full message is read
			let len = Msg::len(self.buf[0]);
			if self.current_byte >= len - 1 {
				// reset buffer and return message
				self.current_byte = 0;
				let msg = self.buf.into();
				self.buf = [0; 3];
				return Ok(msg);
			} else {
				// increase byte counter
				self.current_byte = (self.current_byte + 1) % 3;
			}
		}
		// we need more bits
		Err(nb::Error::WouldBlock)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn msg_motor_power() {
		let buf = [0x23, 0b00001111, 0x00];
		let msg: Msg = buf.into();
		assert_eq!(msg, Msg::MotorPower(15));
	}

	#[test]
	fn msg_speed_diff() {
		let buf = [0x22, 0b11111000, 0x00];
		let msg: Msg = buf.into();
		assert_eq!(msg, Msg::SpeedDiff(-8));
	}

	#[test]
	fn msg_trigger() {
		let buf = [0x21, 0x01, 0x00];
		let msg: Msg = buf.into();
		assert_eq!(msg, Msg::TriggerPulse);
	}
}
