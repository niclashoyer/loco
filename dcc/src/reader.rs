use embedded_hal::digital::InputPin;
use embedded_hal::timer::CountDown;
use embedded_time::duration::*;

use crate::message::Message;
use crate::Error;

const BUF_SIZE: usize = 16;

#[derive(Debug, PartialEq)]
enum State {
	Idle,
}

#[derive(Debug, PartialEq)]
enum Bit {
	Zero,
	One,
	None,
}

/// A reader for the DCC protocol
pub struct Reader<DCC, TIM> {
	pin_dcc: DCC,
	timer: TIM,
	current_byte: u8,
	buf: [u8; BUF_SIZE],
	last_pin_state: bool,
	last_half_bit: Bit,
	bits_read: u8,
	state: State,
}

impl<DCC, TIM> Reader<DCC, TIM>
where
	DCC: InputPin,
	TIM: CountDown,
	TIM::Time: From<Microseconds<u32>>,
{
	pub fn new(pin_dcc: DCC, timer: TIM) -> Self {
		let last_pin_state = pin_dcc.try_is_high().unwrap_or(false);
		Self {
			pin_dcc,
			timer,
			current_byte: 0,
			buf: [0; BUF_SIZE],
			last_pin_state,
			bits_read: 0,
			state: State::Idle,
			last_half_bit: Bit::None,
		}
	}

	fn start_timeout(&mut self) -> Result<(), Error> {
		self.timer
			.try_start(73.microseconds())
			.map_err(|_| Error::TimerError)?;
		Ok(())
	}

	pub fn read(&mut self) -> nb::Result<Message, Error> {
		let pin_state = self.pin_dcc.try_is_high().unwrap_or(false);
		if pin_state != self.last_pin_state {
			if self.timer.try_wait().is_ok() {
				if self.last_half_bit == Bit::Zero {
					println!("zero");
					self.last_half_bit = Bit::None;
				} else {
					self.last_half_bit = Bit::Zero;
				}
			} else {
				if self.last_half_bit == Bit::One {
					println!("one");
					self.last_half_bit = Bit::None;
				} else {
					self.last_half_bit = Bit::One;
				}
			}

			self.start_timeout()?;
			self.last_pin_state = pin_state;
		}
		Err(nb::Error::WouldBlock)
	}
}
