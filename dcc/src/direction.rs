use loco_core::drive::Direction;

pub trait DccDirection {
    fn from_baseline_byte(byte: u8) -> Direction;
    fn to_baseline_byte(&self) -> u8;
    fn from_advanced_byte(byte: u8) -> Direction;
    fn to_advanced_byte(&self) -> u8;
}

impl DccDirection for Direction {
    #[inline]
    fn from_baseline_byte(byte: u8) -> Direction {
        if byte & 0x20 == 0x20 {
            Direction::Forward
        } else {
            Direction::Backward
        }
    }

    #[inline]
    fn to_baseline_byte(&self) -> u8 {
        match self {
            Direction::Forward => 0x20,
            Direction::Backward => 0x00,
        }
    }

    #[inline]
    fn from_advanced_byte(byte: u8) -> Direction {
        if byte & 0x80 == 0x80 {
            Direction::Forward
        } else {
            Direction::Backward
        }
    }

    #[inline]
    fn to_advanced_byte(&self) -> u8 {
        match self {
            Direction::Forward => 0x80,
            Direction::Backward => 0x00,
        }
    }
}
