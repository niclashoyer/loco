use embedded_hal::digital::ToggleableOutputPin;
use embedded_hal::timer::CountDown;
use embedded_time::duration::*;

use crate::message::Message;
use crate::Error;

const BUF_SIZE: usize = 8;
const PREAMBLE_SIZE: u8 = 14;
// half bit lengths in microseconds
const ONE_HALF_BIT: u32 = 58;
const ZERO_HALF_BIT: u32 = 100;

#[derive(Debug, PartialEq)]
enum State {
	Idle,
	Preamble(u8),
	Zero,
	Writing,
}

pub struct Writer<DCC, TIM> {
	pin_dcc: DCC,
	timer: TIM,
	state: State,
	buf: [u8; BUF_SIZE],
	bytes_to_write: usize,
	bits_written: usize,
	bit_written: bool,
}

impl<DCC, TIM> Writer<DCC, TIM>
where
	DCC: ToggleableOutputPin,
	TIM: CountDown,
	TIM::Time: From<Microseconds<u32>>,
{
	fn start_half_zero(&mut self) -> Result<(), Error> {
		self.pin_dcc.try_toggle().map_err(|_| Error::IOError)?;
		self.timer
			.try_start(ZERO_HALF_BIT.microseconds())
			.map_err(|_| Error::TimerError)
	}

	fn start_half_one(&mut self) -> Result<(), Error> {
		self.pin_dcc.try_toggle().map_err(|_| Error::IOError)?;
		self.timer
			.try_start(ONE_HALF_BIT.microseconds())
			.map_err(|_| Error::TimerError)
	}

	fn start_next_bit(&mut self) -> Result<(), Error> {
		let num = self.bits_written / 8;
		let bit = (self.buf[num] >> (self.bits_written % 8)) == 0x01;
		if self.bit_written {
			self.bits_written += 1;
		}
		self.bit_written = !self.bit_written;
		if bit {
			self.start_half_one()
		} else {
			self.start_half_zero()
		}
	}

	pub fn write(&mut self, msg: &Message) -> nb::Result<(), Error> {
		use State::*;
		if self.state != Idle {
			let _ = self
				.timer
				.try_wait()
				.map_err(|e| e.map(|_| Error::TimerError))?;
		}
		match self.state {
			Idle => {
				self.bytes_to_write = msg.to_buf(&mut self.buf);
				self.state = Preamble(PREAMBLE_SIZE);
				self.start_half_one()?;
			}
			Preamble(left) => {
				let mut left = left;
				if self.bit_written {
					self.bit_written = false;
					left = left - 1;
				}
				if left > 0 {
					self.start_half_one()?;
					self.state = Preamble(left);
				} else {
					self.state = Zero;
					self.start_half_zero()?;
				}
			}
			Zero => {
				if self.bit_written {
					self.state = Writing;
					self.start_next_bit()?;
				} else {
					self.start_half_zero()?;
				}
			}
			Writing => {
				if self.bits_written == self.bytes_to_write * 8 {
					self.state = Idle;
					return Ok(());
				}
				if self.bits_written % 8 == 0 {
					self.state = Zero;
					self.start_half_zero()?;
				} else {
					self.start_next_bit()?;
				}
			}
		}
		Err(nb::Error::WouldBlock)
	}
}
