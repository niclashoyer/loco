use embedded_hal::digital::{InputPin, OutputPin};
use std::convert::Infallible;
use std::sync::Arc;
use std::sync::Mutex;

type PinId = usize;

#[derive(Clone, Debug, PartialEq)]
pub enum WireState {
	Low,
	High,
	Floating,
}

impl Copy for WireState {}

#[derive(Debug)]
struct WireWrapper {
	pub state: Vec<WireState>,
	pub pull: WireState,
}

impl WireWrapper {
	fn new() -> Self {
		Self::new_with_pull(WireState::Floating)
	}

	fn new_with_pull(pull: WireState) -> Self {
		WireWrapper {
			state: vec![],
			pull,
		}
	}
}

#[derive(Clone, Debug)]
pub struct Wire {
	wire: Arc<Mutex<WireWrapper>>,
}

impl Wire {
	pub fn new() -> Self {
		Self::new_with_pull(WireState::Floating)
	}

	pub fn new_with_pull(pull: WireState) -> Self {
		Self {
			wire: Arc::new(Mutex::new(WireWrapper::new_with_pull(pull))),
		}
	}

	pub fn set_state(&mut self, id: PinId, state: WireState) {
		self.wire.lock().unwrap().state[id] = state;
		// check for short circuit
		let _ = self.get_state();
	}

	pub fn get_state(&self) -> WireState {
		use WireState::*;
		let mut s = Floating;
		let wire = self.wire.lock().unwrap();
		for state in wire.state.iter() {
			if *state == Floating {
				continue;
			}
			if s != Floating && *state != Floating && *state != s {
				panic!(format!("short circuit: {:?}", wire.state));
			}
			s = *state;
		}
		if s == WireState::Floating {
			wire.pull
		} else {
			s
		}
	}

	pub fn as_push_pull_pin(&self) -> PushPullPin {
		let mut wire = self.wire.lock().unwrap();
		let id = wire.state.len();
		wire.state.push(WireState::Floating);
		PushPullPin {
			id,
			wire: self.clone(),
		}
	}

	pub fn as_open_drain_pin(&self) -> OpenDrainPin {
		let mut wire = self.wire.lock().unwrap();
		let id = wire.state.len();
		wire.state.push(WireState::Floating);
		OpenDrainPin {
			id,
			wire: self.clone(),
		}
	}

	pub fn as_input_pin(&self) -> InputOnlyPin {
		InputOnlyPin { wire: self.clone() }
	}
}

pub struct InputOnlyPin {
	wire: Wire,
}

impl InputPin for InputOnlyPin {
	type Error = Infallible;

	fn try_is_high(&self) -> Result<bool, Self::Error> {
		Ok(self.wire.get_state() == WireState::High)
	}

	fn try_is_low(&self) -> Result<bool, Self::Error> {
		Ok(self.wire.get_state() == WireState::Low)
	}
}

pub struct PushPullPin {
	wire: Wire,
	id: PinId,
}

impl InputPin for PushPullPin {
	type Error = Infallible;

	fn try_is_high(&self) -> Result<bool, Self::Error> {
		Ok(self.wire.get_state() == WireState::High)
	}

	fn try_is_low(&self) -> Result<bool, Self::Error> {
		Ok(self.wire.get_state() == WireState::Low)
	}
}

impl OutputPin for PushPullPin {
	type Error = Infallible;

	fn try_set_low(&mut self) -> Result<(), Self::Error> {
		self.wire.set_state(self.id, WireState::Low);
		Ok(())
	}

	fn try_set_high(&mut self) -> Result<(), Self::Error> {
		self.wire.set_state(self.id, WireState::High);
		Ok(())
	}
}

pub struct OpenDrainPin {
	wire: Wire,
	id: PinId,
}

impl InputPin for OpenDrainPin {
	type Error = Infallible;

	fn try_is_high(&self) -> Result<bool, Self::Error> {
		Ok(self.wire.get_state() == WireState::High)
	}

	fn try_is_low(&self) -> Result<bool, Self::Error> {
		Ok(self.wire.get_state() == WireState::Low)
	}
}

impl OutputPin for OpenDrainPin {
	type Error = Infallible;

	fn try_set_low(&mut self) -> Result<(), Self::Error> {
		self.wire.set_state(self.id, WireState::Floating);
		Ok(())
	}

	fn try_set_high(&mut self) -> Result<(), Self::Error> {
		self.wire.set_state(self.id, WireState::Low);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use WireState::*;

	#[test]
	fn init() {
		let wire = Wire::new();
		assert_eq!(Floating, wire.get_state());
		let wire = Wire::new_with_pull(High);
		assert_eq!(High, wire.get_state());
	}

	#[test]
	fn pull_up() {
		let wire = Wire::new_with_pull(High);
		let mut pin = wire.as_open_drain_pin();
		assert_eq!(High, wire.get_state());
		assert_eq!(Ok(()), pin.try_set_high());
		assert_eq!(Low, wire.get_state());
	}

	#[test]
	fn pull_down() {
		let wire = Wire::new_with_pull(Low);
		let mut pin = wire.as_push_pull_pin();
		assert_eq!(Low, wire.get_state());
		assert_eq!(Ok(()), pin.try_set_high());
		assert_eq!(High, wire.get_state());
		assert_eq!(Ok(()), pin.try_set_low());
		assert_eq!(Low, wire.get_state());
	}

	#[test]
	fn input() {
		let wire = Wire::new();
		let mut pin_out = wire.as_push_pull_pin();
		let pin_in = wire.as_input_pin();
		assert_eq!(Floating, wire.get_state());
		assert_eq!(Ok(false), pin_in.try_is_high());
		assert_eq!(Ok(false), pin_in.try_is_low());
		assert_eq!(Ok(()), pin_out.try_set_low());
		assert_eq!(Low, wire.get_state());
		assert_eq!(Ok(false), pin_in.try_is_high());
		assert_eq!(Ok(true), pin_in.try_is_low());
		assert_eq!(Ok(()), pin_out.try_set_high());
		assert_eq!(High, wire.get_state());
		assert_eq!(Ok(true), pin_in.try_is_high());
		assert_eq!(Ok(false), pin_in.try_is_low());
	}

	#[test]
	#[should_panic]
	fn short_circuit() {
		let wire = Wire::new();
		let mut pin1 = wire.as_push_pull_pin();
		let mut pin2 = wire.as_push_pull_pin();
		assert_eq!(Ok(()), pin1.try_set_high());
		// this will cause a short circuit and panic
		assert_eq!(Ok(()), pin2.try_set_low());
	}
}
