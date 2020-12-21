//! A sender implementation for the SUSI protocol

use crate::message::Msg;
use crate::Error;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::timer::CountDown;
use embedded_time::duration::*;

const HALF_CLK_PERIOD: u32 = 200;

/// A sender for the SUSI protocol
pub struct Sender<DATA, CLK, TIM> {
	pin_data: DATA,
	pin_clk: CLK,
	timer: TIM,
	state: State,
}

#[derive(Debug, PartialEq)]
enum State {
	Idle,
	Writing {
		buf: [u8; 3],
		len: u8,
		bits_written: u8,
		current_byte: u8,
		last_clk: bool,
	},
	WaitingForAck,
}

impl<DATA, CLK, TIM> Sender<DATA, CLK, TIM>
where
	DATA: InputPin + OutputPin,
	CLK: OutputPin,
	TIM: CountDown,
	TIM::Time: From<Microseconds<u32>>,
{
	/// Create a sender using data and clock lines
	///
	/// * `pin_data` - An InputPin + OutputPin that must be configured
	///                as open drain output. If the pin is set to low,
	///                data can be read. If it is set to high, the line
	///                will be pulled to GND.
	/// * `pin_clk`  - An OutputPin used as the clock line
	///                (the receiver will read on falling edges).
	pub fn new(pin_data: DATA, mut pin_clk: CLK, timer: TIM) -> Self {
		pin_clk
			.try_set_low()
			.unwrap_or_else(|_| panic!("can't init clock line"));
		Self {
			pin_data,
			pin_clk,
			timer,
			state: State::Idle,
		}
	}

	pub fn write(&mut self, msg: Msg) -> nb::Result<(), Error> {
		match self.state {
			State::Idle => {
				let len = msg.len() as u8;
				let buf = msg.to_bytes();
				self.pin_clk.try_set_high().map_err(|_| Error::IOError)?;
				self.timer
					.try_start(HALF_CLK_PERIOD.microseconds())
					.map_err(|_| Error::TimerError)?;
				self.state = State::Writing {
					buf,
					last_clk: true,
					bits_written: 0,
					current_byte: 0,
					len,
				};
				Err(nb::Error::WouldBlock)
			}
			State::Writing {
				buf,
				ref mut last_clk,
				ref mut bits_written,
				ref mut current_byte,
				len,
			} => {
				// TODO: check when we are done sending bits
				if *last_clk {
					// last clk was high, so we just need a falling edge
					// so that receivers can read our bit
					self.pin_clk.try_set_low().map_err(|_| Error::IOError)?;
					*last_clk = false;
					*bits_written += 1;
				} else {
					// last clock was low, so we need to bring it high again
					// and get our data line ready
					self.pin_clk.try_set_high().map_err(|_| Error::IOError)?;
					let byte = buf[*current_byte as usize];
					let mask = 1 << (*bits_written + 1);
					let is_high = byte & mask == mask;
					if is_high {
						self.pin_data.try_set_high().map_err(|_| Error::IOError)?;
					} else {
						self.pin_data.try_set_low().map_err(|_| Error::IOError)?;
					}
				}
				Err(nb::Error::WouldBlock)
			}
			// TODO: implement ACK
			_ => Err(nb::Error::WouldBlock),
		}
	}
}
