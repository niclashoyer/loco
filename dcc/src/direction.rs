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
