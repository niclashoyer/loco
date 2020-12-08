use crate::message::Msg;
use embedded_hal::digital::{InputPin, OutputPin};
use nb;

pub struct Reader<DATA, CLK, ACK> {
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

impl<DATA, CLK, ACK> Reader<DATA, CLK, ACK>
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
