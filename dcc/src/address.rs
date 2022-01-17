use loco_core::{address::Address, mov};

pub trait DccAddress {
    fn from_bytes(bytes: &[u8]) -> Address;
    fn to_buf(&self, buf: &mut [u8]) -> usize;
    fn len(&self) -> usize;
}

impl DccAddress for Address {
    fn from_bytes(bytes: &[u8]) -> Address {
        let num = if bytes[0] & 0xC0 == 0xC0 && bytes[0] & 0x3F != 0x3F {
            u16::from_le_bytes([bytes[0] & 0x3F, bytes[1]])
        } else {
            bytes[0] as u16
        };
        Address { num }
    }

    fn to_buf(&self, buf: &mut [u8]) -> usize {
        if self.num > 127 {
            mov!(buf[0..=1] <- &self.num.to_le_bytes());
            buf[0] |= 0xC0;
            2
        } else {
            buf[0] = self.num as u8;
            1
        }
    }

    fn len(&self) -> usize {
        if self.num > 127 {
            2
        } else {
            1
        }
    }
}
