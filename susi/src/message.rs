use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

#[derive(Debug, PartialEq, FromPrimitive, ToPrimitive)]
pub enum Function {
	F0 = 0,
	F1,
	F2,
	F3,
	F4,
	F5,
	F6,
	F7,
	F8,
	F9,
	F10,
	F11,
	F12,
	F13,
	F14,
	F15,
	F16,
	F17,
	F18,
	F19,
	F20,
	F21,
	F22,
	F23,
	F24,
	F25,
	F26,
	F27,
	F28,
	F29,
	F30,
	F31,
	F32,
	F33,
	F34,
	F35,
	F36,
	F37,
	F38,
	F39,
	F40,
	F41,
	F42,
	F43,
	F44,
	F45,
	F46,
	F47,
	F48,
	F49,
	F50,
	F51,
	F52,
	F53,
	F54,
	F55,
	F56,
	F57,
	F58,
	F59,
	F60,
	F61,
	F62,
	F63,
	F64,
	F65,
	F66,
	F67,
	F68,
}

#[derive(Debug, PartialEq, FromPrimitive, ToPrimitive)]
pub enum FunctionGroupNumber {
	G1 = 1,
	G2,
	G3,
	G4,
	G5,
	G6,
	G7,
	G8,
	G9,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct FunctionGroupData {
	data: u8,
}

#[derive(Debug, PartialEq, FromPrimitive, ToPrimitive)]
pub enum AnalogNumber {
	A0 = 0,
	A1,
	A2,
	A3,
	A4,
	A5,
	A6,
	A7,
}

impl FunctionGroupData {
	fn function_position(f: &Function) -> usize {
		let n = f.to_usize().unwrap();
		match n {
			0 => 4,
			1..=4 => n - 1,
			_ => (n - 5) % 8,
		}
	}

	pub fn get(&self, f: &Function) -> bool {
		let p = Self::function_position(f);
		(self.data >> p) & 0x01 == 0x01
	}

	pub fn set(&mut self, f: &Function, value: bool) {
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

impl From<u8> for FunctionGroupData {
	fn from(data: u8) -> Self {
		Self { data }
	}
}

impl From<FunctionGroupData> for u8 {
	fn from(data: FunctionGroupData) -> u8 {
		data.data
	}
}

#[derive(Debug, PartialEq)]
pub enum Direction {
	Forward,
	Backward,
}

impl Direction {
	fn from_u8(val: u8) -> Self {
		if val & (1 << 7) != 0 {
			Self::Forward
		} else {
			Self::Backward
		}
	}

	fn into_u8(&self) -> u8 {
		match self {
			Direction::Forward => 0x80,
			Direction::Backward => 0x00,
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
	Analog(AnalogNumber, u8),
	FunctionGroup(FunctionGroupNumber, FunctionGroupData),
	BinaryState(u8, bool),
	CVByteCheck {
		addr: u8,
		value: u8,
	},
	CVBitManipulation {
		addr: u8,
		check: bool,
		value: bool,
		position: u8,
	},
	CVByteSet {
		addr: u8,
		value: u8,
	},
	Unknown,
}

static MASK7: u8 = 0b01111111;

impl Msg {
	/// Get the length of a message given as command byte
	///
	/// Returns size in bytes (2 or 3)
	pub fn len_from_byte(cmd: u8) -> usize {
		match cmd {
			0x77 | 0x7B | 0x7F => 3,
			_ => 2,
		}
	}

	/// Get the length of a message
	///
	/// Returns size in bytes (2 or 3)
	pub fn len(&self) -> usize {
		match self {
			// only CV messages are 3 bytes long
			Self::CVByteCheck { .. } | Self::CVBitManipulation { .. } | Self::CVByteSet { .. } => 3,
			_ => 2,
		}
	}

	/// Get if this message needs an ACK
	pub fn needs_ack(&self) -> bool {
		match self {
			// only CV messages need an ACK
			Self::CVByteCheck { .. } | Self::CVBitManipulation { .. } | Self::CVByteSet { .. } => {
				true
			}
			_ => false,
		}
	}

	pub fn from_bytes(bytes: &[u8; 3]) -> Self {
		match bytes {
			&[0, 0, _] => Msg::Noop,
			&[33, 0x01, _] => Msg::TriggerPulse,
			&[34, speed, _] => Msg::SpeedDiff(speed as i8),
			&[35, power, _] => Msg::MotorPower(power as i8),
			&[36, data, _] => Msg::LocomotiveSpeed(Direction::from_u8(data), data & MASK7),
			&[37, data, _] => Msg::ControlSpeed(Direction::from_u8(data), data & MASK7),
			&[38, load, _] => Msg::LocomotiveLoad(load & MASK7),
			&[40..=47, value, _] => {
				Msg::Analog(AnalogNumber::from_u8(bytes[0] - 40).unwrap(), value)
			}
			&[96..=104, data, _] => Msg::FunctionGroup(
				FunctionGroupNumber::from_u8(bytes[0] - 95).unwrap(),
				data.into(),
			),
			&[109, data, _] => Msg::BinaryState(data & MASK7, data & 0x80 == 0x80),
			&[119, addr, value] => {
				if (addr & 0x80) == 0x80 {
					Msg::CVByteCheck { addr, value }
				} else {
					Msg::Unknown
				}
			}
			&[123, addr, data] => {
				if (addr & 0x80) == 0x80 {
					Msg::CVBitManipulation {
						addr,
						check: (data & 0x10) == 0x10,
						value: (data & 0x08) == 0x08,
						position: data & 0x07,
					}
				} else {
					Msg::Unknown
				}
			}
			&[127, addr, value] => {
				if (addr & 0x80) == 0x80 {
					Msg::CVByteSet { addr, value }
				} else {
					Msg::Unknown
				}
			}
			_ => Msg::Unknown,
		}
	}

	pub fn to_bytes(&self) -> [u8; 3] {
		match self {
			Msg::Noop => [0x00, 0x00, 0x00],
			Msg::TriggerPulse => [33, 0x01, 0x00],
			Msg::SpeedDiff(diff) => [34, *diff as u8, 0x00],
			Msg::MotorPower(power) => [35, *power as u8, 0x00],
			Msg::LocomotiveSpeed(dir, speed) => [36, dir.into_u8() | (speed & MASK7), 0x00],
			Msg::ControlSpeed(dir, speed) => [37, dir.into_u8() | (speed & MASK7), 0x00],
			Msg::LocomotiveLoad(load) => [38, load & MASK7, 0x00],
			Msg::Analog(num, value) => [40 + num.to_u8().unwrap(), *value, 0x00],
			Msg::FunctionGroup(num, data) => {
				[95 + num.to_u8().unwrap(), Into::<u8>::into(*data), 0x00]
			}
			Msg::BinaryState(addr, set) => [109, ((*set as u8) << 7) | (addr & MASK7), 0x00],
			Msg::CVByteCheck { addr, value } => [119, 0x80 | (addr & MASK7), *value],
			Msg::CVBitManipulation {
				addr,
				check,
				value,
				position,
			} => [
				123,
				0x80 | (addr & MASK7),
				0xE0 | ((*check as u8) << 4) | ((*value as u8) << 3) | (position & 0x07),
			],
			Msg::CVByteSet { addr, value } => [127, 0x80 | (addr & MASK7), *value],
			Msg::Unknown => [0x00, 0x00, 0x00],
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn msg_motor_power() {
		let buf = [0x23, 0b00001111, 0x00];
		let msg: Msg = Msg::from_bytes(&buf);
		assert_eq!(msg, Msg::MotorPower(15));
	}

	#[test]
	fn msg_speed_diff() {
		let buf = [0x22, 0b11111000, 0x00];
		let msg: Msg = Msg::from_bytes(&buf);
		assert_eq!(msg, Msg::SpeedDiff(-8));
	}

	#[test]
	fn msg_trigger() {
		let buf = [0x21, 0x01, 0x00];
		let msg: Msg = Msg::from_bytes(&buf);
		assert_eq!(msg, Msg::TriggerPulse);
	}

	// test all possible buffer combinations to parse as message
	// and back to a buffer
	#[test]
	fn parse_and_back() {
		for a in 0x00..0xff {
			for b in 0x00..0xff {
				for c in 0x00..0xff {
					let mut buf = [a, b, c];
					let msg = Msg::from_bytes(&buf);
					if msg.len() < 3 {
						buf[2] = 0x00;
					}
					match msg {
						Msg::LocomotiveLoad(_) => buf[1] &= MASK7,
						Msg::CVBitManipulation { .. } => {
							buf[2] |= 0xE0;
						}
						_ => {}
					}
					let buf2 = msg.to_bytes();
					if msg != Msg::Unknown {
						assert_eq!(buf, buf2);
					} else {
						assert_eq!([0x00, 0x00, 0x00], buf2);
					}
				}
			}
		}
	}
}
