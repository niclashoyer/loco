use num_traits::{FromPrimitive, ToPrimitive};

use loco_core::{analog::AnalogNumber, drive::Direction, functions::FunctionGroupNumber};

use loco_dcc::function::FunctionGroupByte;

pub trait Byte<T> {
    fn from_byte(byte: u8) -> T;
    fn to_byte(&self) -> u8;
}

impl Byte<Direction> for Direction {
    fn from_byte(byte: u8) -> Self {
        if byte & (1 << 7) != 0 {
            Self::Forward
        } else {
            Self::Backward
        }
    }

    fn to_byte(&self) -> u8 {
        match self {
            Direction::Forward => 0x80,
            Direction::Backward => 0x00,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Noop,
    TriggerPulse,
    SpeedDiff(i8),
    MotorPower(i8),
    LocomotiveSpeed(Direction, u8),
    ControlSpeed(Direction, u8),
    LocomotiveLoad(u8),
    Analog(AnalogNumber, u8),
    FunctionGroup(FunctionGroupNumber, FunctionGroupByte),
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
    Unknown([u8; 3]),
}

static MASK7: u8 = 0b01111111;

#[allow(clippy::len_without_is_empty)]
impl Msg {
    /// Get the length of a message given as command byte
    ///
    /// Returns size in bytes (2 or 3)
    pub fn len_from_byte(cmd: u8) -> u8 {
        match cmd {
            0x77 | 0x7B | 0x7F => 3,
            _ => 2,
        }
    }

    /// Get the length of a message
    ///
    /// Returns size in bytes (2 or 3)
    pub fn len(&self) -> u8 {
        match self {
            // only CV messages are 3 bytes long
            Self::CVByteCheck { .. } | Self::CVBitManipulation { .. } | Self::CVByteSet { .. } => 3,
            // unknown is always full length, since we don't know
            Self::Unknown(_) => 3,
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
        match *bytes {
            [0, 0, _] => Msg::Noop,
            [33, 0x01, _] => Msg::TriggerPulse,
            [34, speed, _] => Msg::SpeedDiff(speed as i8),
            [35, power, _] => Msg::MotorPower(power as i8),
            [36, data, _] => Msg::LocomotiveSpeed(Direction::from_byte(data), data & MASK7),
            [37, data, _] => Msg::ControlSpeed(Direction::from_byte(data), data & MASK7),
            [38, load, _] => Msg::LocomotiveLoad(load & MASK7),
            [40..=47, value, _] => {
                Msg::Analog(AnalogNumber::from_u8(bytes[0] - 40).unwrap(), value)
            }
            [96..=104, data, _] => Msg::FunctionGroup(
                FunctionGroupNumber::from_u8(bytes[0] - 95).unwrap(),
                data.into(),
            ),
            [109, data, _] => Msg::BinaryState(data & MASK7, data & 0x80 == 0x80),
            [119, addr, value] => {
                if (addr & 0x80) == 0x80 {
                    Msg::CVByteCheck { addr, value }
                } else {
                    Msg::Unknown(*bytes)
                }
            }
            [123, addr, data] => {
                if (addr & 0x80) == 0x80 {
                    Msg::CVBitManipulation {
                        addr,
                        check: (data & 0x10) == 0x10,
                        value: (data & 0x08) == 0x08,
                        position: data & 0x07,
                    }
                } else {
                    Msg::Unknown(*bytes)
                }
            }
            [127, addr, value] => {
                if (addr & 0x80) == 0x80 {
                    Msg::CVByteSet { addr, value }
                } else {
                    Msg::Unknown(*bytes)
                }
            }
            _ => Msg::Unknown(*bytes),
        }
    }

    pub fn to_bytes(&self) -> [u8; 3] {
        match self {
            Msg::Noop => [0x00, 0x00, 0x00],
            Msg::TriggerPulse => [33, 0x01, 0x00],
            Msg::SpeedDiff(diff) => [34, *diff as u8, 0x00],
            Msg::MotorPower(power) => [35, *power as u8, 0x00],
            Msg::LocomotiveSpeed(dir, speed) => [36, dir.to_byte() | (speed & MASK7), 0x00],
            Msg::ControlSpeed(dir, speed) => [37, dir.to_byte() | (speed & MASK7), 0x00],
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
            Msg::Unknown(bytes) => *bytes,
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
                    assert_eq!(buf, buf2);
                }
            }
        }
    }

    // test function group decoding and manipulation
    #[test]
    fn function_group_data() {
        use loco_core::functions::Function::*;
        let data: u8 = 0b1010_1010;
        let mut group: FunctionGroupByte = data.into();

        assert!(!group.get(F0)); // F0 should be at index 4
        assert!(group.get(F4));

        assert!(!group.get(F5));
        assert!(group.get(F6));
        assert!(!group.get(F7));
        assert!(group.get(F8));
        assert!(!group.get(F9));
        assert!(group.get(F10));
        assert!(!group.get(F11));
        assert!(group.get(F12));
        group.set(F9, true);
        group.set(F10, false);
        let data: u8 = group.into();
        assert_eq!(data, 0b1001_1010);
        group.clear();
        let data: u8 = group.into();
        assert_eq!(data, 0x00);
    }

    // test for messages that need ack
    #[test]
    fn needs_ack() {
        let msg = Msg::Unknown([0; 3]);
        assert!(!msg.needs_ack());
        let msg = Msg::LocomotiveLoad(127);
        assert!(!msg.needs_ack());
        let msg = Msg::CVByteCheck {
            addr: 127,
            value: 0xAA,
        };
        assert!(msg.needs_ack());
        let msg = Msg::CVBitManipulation {
            addr: 222,
            check: false,
            value: true,
            position: 5,
        };
        assert!(msg.needs_ack());
        let msg = Msg::CVByteSet {
            addr: 130,
            value: 0xBB,
        };
        assert!(msg.needs_ack());
    }
}
