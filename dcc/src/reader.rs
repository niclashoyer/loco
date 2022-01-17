use embedded_hal::digital::blocking::InputPin;
use embedded_hal::timer::nb::CountDown;
use embedded_time::duration::*;
use log::{debug, trace};

use crate::message::Message;
use crate::Error;

const BUF_SIZE: usize = 8;

#[derive(Debug, PartialEq)]
enum State {
	Idle,
	Byte,
	StartBit,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Bit {
	Zero,
	One,
}

impl Copy for Bit {}

impl From<Bit> for bool {
	#[inline]
	fn from(bit: Bit) -> bool {
		bit == Bit::One
	}
}

impl From<Bit> for u8 {
	#[inline]
	fn from(bit: Bit) -> u8 {
		use Bit::*;
		match bit {
			One => 0x01,
			Zero => 0x00,
		}
	}
}

pub trait Decoder {
	fn decode(&mut self) -> nb::Result<Bit, Error>;
}

#[derive(Debug)]
pub struct PinDecoder<DCC, TIM> {
	pin_dcc: DCC,
	timer: TIM,
	last_half_bit: Option<Bit>,
	last_pin_state: bool,
}

impl<DCC, TIM> PinDecoder<DCC, TIM>
where
	DCC: InputPin,
	TIM: CountDown,
	TIM::Time: From<Microseconds<u32>>,
{
	fn start_timeout(&mut self) -> Result<(), Error> {
		self.timer
			.start(73.microseconds())
			.map_err(|_| Error::TimerError)?;
		Ok(())
	}

	pub fn new(pin_dcc: DCC, timer: TIM) -> Self {
		let last_pin_state = pin_dcc.is_high().unwrap_or(false);
		Self {
			pin_dcc,
			timer,
			last_half_bit: None,
			last_pin_state,
		}
	}
}

impl<DCC, TIM> Decoder for PinDecoder<DCC, TIM>
where
	DCC: InputPin,
	TIM: CountDown,
	TIM::Time: From<Microseconds<u32>>,
{
	fn decode(&mut self) -> nb::Result<Bit, Error> {
		let mut ret = None;
		let pin_state = self.pin_dcc.is_high().unwrap_or(false);
		if pin_state != self.last_pin_state {
			if self.timer.wait().is_ok() {
				ret = self.handle_half_bit(Bit::Zero);
			} else {
				ret = self.handle_half_bit(Bit::One);
			}
			self.start_timeout()?;
			self.last_pin_state = pin_state;
		}
		if let Some(bit) = ret {
			Ok(bit)
		} else {
			Err(nb::Error::WouldBlock)
		}
	}
}

impl<DCC, TIM> PinDecoder<DCC, TIM>
where
	DCC: InputPin,
	TIM: CountDown,
	TIM::Time: From<Microseconds<u32>>,
{
	#[inline]
	fn handle_half_bit(&mut self, bit: Bit) -> Option<Bit> {
		trace!(
			"{:<20}{:<20}",
			format!("edge detected"),
			format!("{:?}/{:?}", self.last_half_bit, bit)
		);
		if self.last_half_bit == Some(bit) {
			self.last_half_bit = None;
			Some(bit)
		} else {
			self.last_half_bit = Some(bit);
			None
		}
	}
}

/// A reader for the DCC protocol
pub struct Reader<D> {
	decoder: D,
	one_bits: u8,
	current_byte: u8,
	buf: [u8; BUF_SIZE],
	bits_read: u8,
	state: State,
}

impl<D> Reader<D>
where
	D: Decoder,
{
	pub fn new(decoder: D) -> Self {
		Self {
			decoder,
			current_byte: 0,
			one_bits: 0,
			buf: [0; BUF_SIZE],
			bits_read: 0,
			state: State::Idle,
		}
	}

	fn reset(&mut self) {
		use State::*;
		self.state = Idle;
		self.bits_read = 0;
		self.current_byte = 0;
		self.buf = [0; BUF_SIZE];
	}

	fn start(&mut self) {
		use State::*;
		self.state = Byte;
		self.bits_read = 0;
		self.one_bits = 0;
		self.current_byte = 0;
		self.buf = [0; BUF_SIZE];
	}

	pub fn read(&mut self) -> nb::Result<Message, Error> {
		use Bit::*;
		use State::*;
		let bit = self.decoder.decode()?;
		trace!(
			"{:<20}{:<20}",
			"bit read",
			format!("{:?}/{:?}/{:?}", self.state, bit, self.bits_read)
		);
		match bit {
			One => {
				self.one_bits += 1;
			}
			Zero => {
				if self.one_bits > 9 {
					debug!("detected preamble + zero, start reading bits");
					self.start();
					return Err(nb::Error::WouldBlock);
				}
				self.one_bits = 0;
			}
		}
		match self.state {
			Byte => {
				let i = self.current_byte as usize;
				let data: u8 = bit.into();
				self.buf[i] <<= 1;
				self.buf[i] |= data;
				self.bits_read += 1;
				if self.bits_read == 8 {
					debug!("read byte {:#04x}", self.buf[i]);
					self.bits_read = 0;
					self.current_byte += 1;
					self.state = StartBit;
				}
			}
			StartBit => {
				if bit == Zero {
					self.state = Byte;
				} else {
					let len = self.current_byte as usize;
					let msg = Message::from_bytes(&self.buf[..len]);
					debug!("read bytes {:#04X?} as {:?}", &self.buf[..len], msg);
					self.reset();
					return Ok(msg);
				}
			}
			_ => {}
		}
		Err(nb::Error::WouldBlock)
	}
}
