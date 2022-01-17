//! A writer implementation for the SUSI protocol

use crate::message::Msg;
use crate::Error;
use embedded_hal::digital::blocking::{InputPin, OutputPin};
use embedded_hal::timer::nb::CountDown;
use embedded_time::duration::*;

const HALF_CLK_PERIOD: u32 = 200;

/// A writer for the SUSI protocol
pub struct Writer<DATA, CLK, TIM> {
	pin_data: DATA,
	pin_clk: CLK,
	timer: TIM,
	buf: [u8; 3],
	last_clk: bool,
	bits_written: u8,
	len: u8,
	state: State,
}

#[derive(Debug, PartialEq)]
pub enum WriterResult {
	None,
	Ack,
	Nack,
}

#[derive(Debug, PartialEq)]
enum State {
	Idle,
	Writing,
	Waiting,
	WaitingForAck,
	#[allow(dead_code)] // still unimplemented
	WaitingForReset,
}

impl<DATA, CLK, TIM> Writer<DATA, CLK, TIM>
where
	DATA: InputPin + OutputPin,
	CLK: OutputPin,
	TIM: CountDown,
	TIM::Time: From<Microseconds<u32>>,
{
	/// Create a writer using data and clock lines
	///
	/// * `pin_data` - An InputPin + OutputPin that must be configured
	///                as open drain output. If the pin is set to low,
	///                the connection will be open and pulled up by an
	///                external pull up. If it is set to high,
	///                the line will be pulled down.
	/// * `pin_clk`  - An OutputPin used as the clock line
	///                (the receiver will read on falling edges).
	pub fn new(pin_data: DATA, mut pin_clk: CLK, timer: TIM) -> Self {
		pin_clk
			.set_low()
			.unwrap_or_else(|_| panic!("can't init clock line"));
		Self {
			pin_data,
			pin_clk,
			timer,
			buf: [0; 3],
			last_clk: false,
			bits_written: 0,
			len: 0,
			state: State::Idle,
		}
	}

	fn reset(&mut self) {
		self.buf = [0; 3];
		self.bits_written = 0;
		self.state = State::Idle;
	}

	pub fn write(&mut self, msg: &Msg) -> nb::Result<WriterResult, Error> {
		match self.state {
			State::Idle => {
				self.buf = msg.to_bytes();
				self.len = msg.len() as u8;
				self.state = State::Writing;
				Err(nb::Error::WouldBlock)
			}
			State::Writing | State::Waiting => {
				if self.state == State::Waiting {
					self.timer
						.wait()
						.map_err(|e| e.map(|_| Error::TimerError))?;
					self.state = State::Writing;
					if self.bits_written == self.len * 8 {
						if msg.needs_ack() {
							self.state = State::WaitingForAck;
						} else {
							self.reset();
							return Ok(WriterResult::None);
						}
					}
				}
				if self.last_clk {
					// last clk was high, so we just need a falling edge
					// so that receivers can read our bit
					self.pin_clk.set_low().map_err(|_| Error::IOError)?;
					self.last_clk = false;
					self.bits_written += 1;
					self.timer
						.start(HALF_CLK_PERIOD.microseconds())
						.map_err(|_| Error::TimerError)?;
					self.state = State::Waiting;
				} else {
					// last clock was low, so we need to bring it high again
					// and get our data line ready
					self.pin_clk.set_high().map_err(|_| Error::IOError)?;
					self.last_clk = true;
					let byte = self.buf[(self.bits_written / 8) as usize];
					let mask = 1 << (self.bits_written % 8);
					let is_high = byte & mask == mask;
					// open drain output: setting to low will write "1" using
					// external pull up. Setting to high will write "0" using
					// internal connection to GND (pulling low).
					if is_high {
						self.pin_data.set_low().map_err(|_| Error::IOError)?;
					} else {
						self.pin_data.set_high().map_err(|_| Error::IOError)?;
					}
					self.timer
						.start(HALF_CLK_PERIOD.microseconds())
						.map_err(|_| Error::TimerError)?;
					self.state = State::Waiting;
				}
				Err(nb::Error::WouldBlock)
			}
			State::WaitingForAck => {
				panic!("ACK not implemented")
			}
			State::WaitingForReset => {
				// TODO: implement reset delay after 20 commands (as per spec)
				Err(nb::Error::WouldBlock)
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{Writer, WriterResult};
	use crate::message::Msg;
	use crate::tests_mock::*;
	use loco_core::drive::Direction;

	// convert a vector of bytes to mocked pins that can be used
	// to test a writer
	fn get_pin_states(msg: &Msg) -> (Mock, Mock, MockTimer, usize) {
		let word = msg.to_bytes();
		let bytes = msg.len() as usize;
		let bits = bytes * 8;
		// add pin states for data line
		let mut data_states = vec![];
		for byte in word.iter() {
			for j in 0..8 {
				// open drain output: high / low is inversed
				if (byte >> j) & 0x01 == 1 {
					data_states.push(Transaction::set(State::Low));
				} else {
					data_states.push(Transaction::set(State::High));
				}
			}
		}
		// add pin states for data line
		if msg.needs_ack() {
			data_states.push(Transaction::get(State::High));
			data_states.push(Transaction::get(State::Low));
		}
		let data = Mock::new(&data_states);
		// add pin states for clock line
		let mut clk_states = vec![];
		clk_states.push(Transaction::set(State::Low));
		for _i in 0..bits {
			clk_states.push(Transaction::set(State::High));
			clk_states.push(Transaction::set(State::Low));
		}
		let clk = Mock::new(&clk_states);
		// add a mocked timer
		let timer = MockTimer::new(128_000u64.Hz());
		(data, clk, timer, bits)
	}

	// test writing a single NOOP message
	#[test]
	fn single_noop() {
		let msg = Msg::Noop;
		let (data, clk, timer, _bits) = get_pin_states(&msg);
		let mut writer = Writer::new(data, clk, timer);
		let res = nb::block!(writer.write(&msg));
		assert_eq!(res, Ok(WriterResult::None));
	}

	// test writing a single speed message
	#[test]
	fn single_speed() {
		let msg = Msg::LocomotiveSpeed(Direction::Forward, 120);
		let (data, clk, timer, _bits) = get_pin_states(&msg);
		let mut writer = Writer::new(data, clk, timer);
		let res = nb::block!(writer.write(&msg));
		assert_eq!(res, Ok(WriterResult::None));
	}
}
