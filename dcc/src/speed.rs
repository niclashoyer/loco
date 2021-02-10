use loco_core::drive::Speed;

pub trait DccSpeed {
	fn from_byte_14_steps(byte: u8) -> Speed;
	fn from_byte_28_steps(byte: u8) -> Speed;
	fn from_byte_128_steps(byte: u8) -> Speed;
	fn to_byte(&self) -> u8;
}

impl DccSpeed for Speed {
	#[inline]
	fn from_byte_14_steps(byte: u8) -> Speed {
		use Speed::*;
		match byte & 0x0F {
			0x00 => Stop,
			0x01 => EmergencyStop,
			s => Steps14(s * 8),
		}
	}

	#[inline]
	fn from_byte_28_steps(byte: u8) -> Speed {
		use Speed::*;
		match byte & 0x1F {
			0x00 => Stop,
			0x01 => EmergencyStop,
			s => Steps28(((s << 1) | ((byte >> 4) & 0x01)) * 4),
		}
	}

	#[inline]
	fn from_byte_128_steps(byte: u8) -> Speed {
		use Speed::*;
		match byte & 0x7F {
			0x00 => Stop,
			0x01 => EmergencyStop,
			_ => Steps128((byte & 0x7F) * 2),
		}
	}

	#[inline]
	fn to_byte(&self) -> u8 {
		use Speed::*;
		match self {
			Stop => 0x00,
			EmergencyStop => 0x01,
			Steps14(s) => (s / 4) & 0x0F,
			Steps28(s) => (((s / 8) & 0x0F) << 1) | (((s / 4) & 0x01) << 4),
			Steps128(s) => (s / 2) & 0x7F,
		}
	}
}
