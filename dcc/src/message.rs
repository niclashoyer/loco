use crate::{address::DccAddress, direction::DccDirection, speed::DccSpeed};
use loco_core::{
    address::Address,
    drive::{Direction, Speed},
};
use log::trace;

#[derive(Debug, Clone, PartialEq)]
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
        let cmd = (bytes[0] & 0b111_00000) >> 5;
        match cmd {
            0b010 | 0b011 => Drive(
                addr,
                Direction::from_baseline_byte(bytes[0]),
                Speed::from_byte_28_steps(bytes[0]),
            ),
            0b001 => match bytes[0] & 0b000_11111 {
                0b11111 => Drive(
                    addr,
                    Direction::from_advanced_byte(bytes[1]),
                    Speed::from_byte_128_steps(bytes[1]),
                ),
                _ => Unknown(addr),
            },
            _ => Unknown(addr),
        }
    }

    pub fn to_buf(&self, buf: &mut [u8]) -> usize {
        use loco_core::add_xor;
        use Message::*;
        match self {
            Drive(addr, dir, speed) => {
                let n = addr.to_buf(buf);
                if let Speed::Steps128(_) = speed {
                    buf[n] = 0b001_11111;
                    buf[n + 1] = dir.to_advanced_byte() | speed.to_byte();
                    add_xor(buf, n + 3)
                } else {
                    buf[n] = 0b010_00000 | dir.to_baseline_byte() | speed.to_byte();
                    add_xor(buf, n + 2)
                }
            }
            _ => unimplemented!(),
        }
    }
}
