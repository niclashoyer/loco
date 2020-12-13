//! A client implementation for the SUSI protocol

use crate::message::Msg;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::timer::{Cancel, CountDown};
use embedded_time::duration::*;
use nb;

/// A Receiver for the SUSI protocol
pub struct Receiver<DATA, CLK, ACK, TIM> {
	pin_data: DATA,
	pin_clk: CLK,
	pin_ack: ACK,
	timer: TIM,
	current_byte: usize,
	buf: [u8; 3],
	last_clk: bool,
	bits_read: u8,
	state: State,
}

#[derive(Debug, PartialEq)]
pub enum State {
	Idle,
	WaitAcknowledge,
	WaitAfterByte,
}

/// Errors returned by the Receiver
#[derive(Debug, PartialEq)]
pub enum Error {
	IOError,
	TimerError,
}

impl<DATA, CLK, ACK, TIM> Receiver<DATA, CLK, ACK, TIM>
where
	DATA: InputPin,
	CLK: InputPin,
	ACK: OutputPin,
	TIM: CountDown + Cancel,
	TIM::Time: From<Milliseconds<u32>>,
{
	/// Create a receiver using data, clock and ack lines
	///
	/// * `pin_data` - An InputPin used to read the data line
	/// * `pin_clk`  - An InputPin used to read the clock line
	///                (falling edge reads a bit from `pin_data`)
	/// * `pin_ack`  - An OutputPin used to send an acknowledge
	///                (setting this to high should pull the ack
	///                 line down, e.g. using an open drain output)
	pub fn new(pin_data: DATA, pin_clk: CLK, pin_ack: ACK, timer: TIM) -> Self {
		let last_clk = pin_clk.try_is_high().unwrap_or(false);
		Self {
			pin_data,
			pin_clk,
			pin_ack,
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
		self.state = State::Idle;
	}

	fn start_timeout(&mut self) -> Result<(), Error> {
		self.state = State::WaitAfterByte;
		self.timer
			.try_start(8u32.milliseconds())
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
		if self.state != State::Idle {
			if self.timer.try_wait().is_ok() {
				self.reset();
			}
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
			self.buf[self.current_byte] |= data << self.bits_read;
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
			if self.timer.try_wait().is_ok() {
				self.pin_ack.try_set_low().map_err(|_| Error::IOError)?;
				self.reset();
				Ok(())
			} else {
				Err(nb::Error::WouldBlock)
			}
		} else {
			self.timer
				.try_start(2u32.milliseconds())
				.map_err(|_| Error::TimerError)?;
			self.pin_ack.try_set_high().map_err(|_| Error::IOError)?;
			self.state = State::WaitAcknowledge;
			Err(nb::Error::WouldBlock)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::message::Msg;
	use embedded_hal_mock::pin;
	use pin::{Mock, State, Transaction};

	use embedded_hal::timer::{Cancel, CountDown};
	use embedded_time::rate::*;
	struct MockTimer {
		clock: Hertz<u64>,
		count: u64,
	}

	impl MockTimer {
		pub fn new(clock: Hertz<u64>) -> Self {
			MockTimer { clock, count: 0 }
		}
	}

	impl CountDown for MockTimer {
		type Error = ();
		type Time = Nanoseconds<u64>;

		fn try_start<T: Into<Nanoseconds<u64>>>(&mut self, timeout: T) -> Result<(), Self::Error> {
			self.count = timeout.into().0 * self.clock.0 / 1_000_000_000_u64;
			Ok(())
		}

		fn try_wait(&mut self) -> nb::Result<(), Self::Error> {
			if self.count > 0 {
				self.count -= 1;
			}
			if self.count > 0 {
				Err(nb::Error::WouldBlock)
			} else {
				Ok(())
			}
		}
	}

	impl Cancel for MockTimer {
		fn try_cancel(&mut self) -> Result<(), Self::Error> {
			if self.count > 0 {
				self.count = 0;
				Ok(())
			} else {
				Err(())
			}
		}
	}

	// convert a vector of bytes to mocked pins that can be used
	// to test a receiver
	fn get_pin_states(word: Vec<u8>) -> (Mock, Mock, Mock, MockTimer, usize) {
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
		let data = Mock::new(&data_states);
		// add pin states for clock line
		let mut clk_states = vec![];
		clk_states.push(Transaction::get(State::Low));
		for _i in 0..bits {
			clk_states.push(Transaction::get(State::High));
			clk_states.push(Transaction::get(State::Low));
		}
		let clk = Mock::new(&clk_states);
		// add pin states for ack line
		let ack = Mock::new(vec![]);
		// add a mocked timer
		let timer = MockTimer::new(128_000u64.Hz());
		(data, clk, ack, timer, bits)
	}

	// test reading a single NOOP message
	#[test]
	fn single_noop() {
		let (data, clk, ack, timer, bits) = get_pin_states(vec![0x00, 0x00]);
		let mut receiver = Receiver::new(data, clk, ack, timer);
		for i in 0..bits * 2 {
			let res = receiver.read();
			if i < (bits * 2) - 1 {
				assert_eq!(res, Err(nb::Error::WouldBlock));
			} else {
				assert_eq!(res, Ok(Msg::Noop));
			}
		}
	}

	// test reading a single speed message
	#[test]
	fn single_diff() {
		let (data, clk, ack, timer, bits) = get_pin_states(vec![0x22, 0xf8]);
		let mut receiver = Receiver::new(data, clk, ack, timer);
		for i in 0..bits * 2 {
			let res = receiver.read();
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
		let (data, clk, ack, timer, _bits) =
			get_pin_states(vec![0x22, 0xf8, 0x00, 0x00, 0x23, 0x08]);
		let mut receiver = Receiver::new(data, clk, ack, timer);
		for i in 0..32 {
			let res = receiver.read();
			if i < 31 {
				assert_eq!(res, Err(nb::Error::WouldBlock));
			} else {
				assert_eq!(res, Ok(Msg::SpeedDiff(-8)));
			}
		}
		for i in 0..32 {
			let res = receiver.read();
			if i < 31 {
				assert_eq!(res, Err(nb::Error::WouldBlock));
			} else {
				assert_eq!(res, Ok(Msg::Noop));
			}
		}
		for i in 0..32 {
			let res = receiver.read();
			if i < 31 {
				assert_eq!(res, Err(nb::Error::WouldBlock));
			} else {
				assert_eq!(res, Ok(Msg::MotorPower(8)));
			}
		}
	}
}
