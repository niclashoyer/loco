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

impl Msg {
	pub fn len(cmd: u8) -> usize {
		match cmd {
			0x77 | 0x7B | 0x7F => 3,
			_ => 2,
		}
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
