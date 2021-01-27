//! A client implementation for the SUSI protocol

use crate::message::Msg;
use crate::Error;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::timer::CountDown;
use embedded_time::duration::*;

/// A reader for the SUSI protocol
pub struct Reader<DATA, CLK, TIM> {
	pin_data: DATA,
	pin_clk: CLK,
	timer: TIM,
	current_byte: u8,
	buf: [u8; 3],
	last_clk: bool,
	bits_read: u8,
	state: State,
}

#[derive(Debug, PartialEq)]
enum State {
	Idle,
	WaitAcknowledge,
	WaitAfterByte,
}

impl<DATA, CLK, TIM> Reader<DATA, CLK, TIM>
where
	DATA: InputPin + OutputPin,
	CLK: InputPin,
	TIM: CountDown,
	TIM::Time: From<Milliseconds<u32>>,
{
	/// Create a reader using data and clock lines
	///
	/// * `pin_data` - An InputPin + OutputPin that must be configured
	///                as open drain output. If the pin is set to low,
	///                the connection will be open and pulled up by an
	///                external pull up. If it is set to high,
	///                the line will be pulled down.
	/// * `pin_clk`  - An InputPin used to read the clock line
	///                (falling edge reads a bit from `pin_data`)
	pub fn new(pin_data: DATA, pin_clk: CLK, timer: TIM) -> Self {
		let last_clk = pin_clk.try_is_high().unwrap_or(false);
		Self {
			pin_data,
			pin_clk,
			timer,
			current_byte: 0,
			buf: [0; 3],
			last_clk,
			bits_read: 0,
			state: State::Idle,
		}
	}

	fn reset(&mut self) {
		self.buf = [0; 3];
		self.bits_read = 0;
		self.current_byte = 0;
		self.state = State::Idle;
	}

	fn start_timeout(&mut self) -> Result<(), Error> {
		self.state = State::WaitAfterByte;
		self.timer
			.try_start(8.milliseconds())
			.map_err(|_| Error::TimerError)?;
		Ok(())
	}

	pub fn read(&mut self) -> nb::Result<Msg, Error> {
		// if we are waiting to finish an acknowledge,
		// call `ack` method and only continue if it won't
		// block anymore
		if self.state == State::WaitAcknowledge {
			let _ = self.ack()?;
		}
		// if we are not in idle state, check if the timer
		// finished to sync again
		if self.state == State::WaitAfterByte && self.timer.try_wait().is_ok() {
			println!("reset");
			self.reset();
		}
		// get current clock signal
		let clk = self.pin_clk.try_is_high().map_err(|_| Error::IOError)?;
		// check if we have a falling edge
		if self.last_clk && !clk {
			if self.state == State::Idle {
				// handle 8ms sync timeout
				self.start_timeout()?;
			}
			// read data on falling edge
			let data = if self.pin_data.try_is_high().map_err(|_| Error::IOError)? {
				1
			} else {
				0
			};
			// push bit into buffer
			self.buf[self.current_byte as usize] |= data << self.bits_read;
			self.bits_read += 1;
		}
		// save clock signal to detect next falling edge
		self.last_clk = clk;
		// full byte read
		if self.bits_read == 8 {
			// handle 8ms sync timeout
			self.start_timeout()?;
			// prepare to read the next byte
			self.bits_read = 0;
			// check if full message is read
			let len = Msg::len_from_byte(self.buf[0]);
			if self.current_byte >= len - 1 {
				// reset buffer and return message
				self.bits_read = 0;
				self.current_byte = 0;
				let msg = Msg::from_bytes(&self.buf);
				self.buf = [0; 3];
				self.state = State::WaitAfterByte;
				return Ok(msg);
			} else {
				// increase byte counter
				self.current_byte = (self.current_byte + 1) % 3;
			}
		}
		// we need more bits
		Err(nb::Error::WouldBlock)
	}

	pub fn ack(&mut self) -> nb::Result<(), Error> {
		if self.state == State::WaitAcknowledge {
			self.timer.try_wait().map_err(|_| Error::IOError)?;
			self.pin_data.try_set_low().map_err(|_| Error::IOError)?;
			self.reset();
			Ok(())
		} else {
			self.timer
				.try_start(2.milliseconds())
				.map_err(|_| Error::TimerError)?;
			self.pin_data.try_set_high().map_err(|_| Error::IOError)?;
			self.state = State::WaitAcknowledge;
			Err(nb::Error::WouldBlock)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::Reader;
	use crate::message::Msg;
	use crate::tests_mock::*;

	// convert a vector of bytes to mocked pins that can be used
	// to test a reader
	fn get_pin_states(word: Vec<u8>, acks: usize) -> (Mock, Mock, MockTimer, usize) {
		let bytes = word.len();
		let bits = bytes * 8;
		// add pin states for data line
		let mut data_states = vec![];
		for i in 0..bytes {
			for j in 0..8 {
				if (word[i] >> j) & 0x01 == 1 {
					data_states.push(Transaction::get(State::High));
				} else {
					data_states.push(Transaction::get(State::Low));
				}
			}
		}
		// add pin states for data line
		for _ in 1..=acks {
			data_states.push(Transaction::set(State::High));
			data_states.push(Transaction::set(State::Low));
		}
		let data = Mock::new(&data_states);
		// add pin states for clock line
		let mut clk_states = vec![];
		clk_states.push(Transaction::get(State::Low));
		for _i in 0..bits {
			clk_states.push(Transaction::get(State::High));
			clk_states.push(Transaction::get(State::Low));
		}
		let clk = Mock::new(&clk_states);
		// add a mocked timer
		let timer = MockTimer::new(128_000u64.Hz());
		(data, clk, timer, bits)
	}

	// test reading a single NOOP message
	#[test]
	fn single_noop() {
		let (data, clk, timer, bits) = get_pin_states(vec![0x00, 0x00], 0);
		let mut reader = Reader::new(data, clk, timer);
		for i in 0..bits * 2 {
			let res = reader.read();
			if i < (bits * 2) - 1 {
				assert_eq!(res, Err(nb::Error::WouldBlock));
			} else {
				assert_eq!(res, Ok(Msg::Noop));
			}
		}
	}

	// test reading a single speed differenc differencee message
	#[test]
	fn single_diff() {
		let (data, clk, timer, bits) = get_pin_states(vec![0x22, 0xf8], 0);
		let mut reader = Reader::new(data, clk, timer);
		for i in 0..bits * 2 {
			let res = reader.read();
			if i < (bits * 2) - 1 {
				assert_eq!(res, Err(nb::Error::WouldBlock));
			} else {
				assert_eq!(res, Ok(Msg::SpeedDiff(-8)));
			}
		}
	}

	// test reading three consecutive messages
	#[test]
	fn three_messages() {
		let (data, clk, timer, _bits) = get_pin_states(vec![0x22, 0xf8, 0x00, 0x00, 0x23, 0x08], 0);
		let mut reader = Reader::new(data, clk, timer);
		for i in 0..32 {
			let res = reader.read();
			if i < 31 {
				assert_eq!(res, Err(nb::Error::WouldBlock));
			} else {
				assert_eq!(res, Ok(Msg::SpeedDiff(-8)));
			}
		}
		for i in 0..32 {
			let res = reader.read();
			if i < 31 {
				assert_eq!(res, Err(nb::Error::WouldBlock));
			} else {
				assert_eq!(res, Ok(Msg::Noop));
			}
		}
		for i in 0..32 {
			let res = reader.read();
			if i < 31 {
				assert_eq!(res, Err(nb::Error::WouldBlock));
			} else {
				assert_eq!(res, Ok(Msg::MotorPower(8)));
			}
		}
	}

	#[test]
	fn cv_set() {
		let (data, clk, timer, bits) = get_pin_states(vec![0x7F, 0x80, 0xAA], 1);
		let mut reader = Reader::new(data, clk, timer);
		for i in 0..bits * 2 {
			let res = reader.read();
			if i < (bits * 2) - 1 {
				assert_eq!(res, Err(nb::Error::WouldBlock));
			} else {
				assert_eq!(
					res,
					Ok(Msg::CVByteSet {
						addr: 0x80,
						value: 0xAA
					})
				);
			}
		}
		let _ = reader.ack();
	}
}
