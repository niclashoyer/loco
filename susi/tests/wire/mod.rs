use embedded_hal::digital::{InputPin, OutputPin};
use std::convert::Infallible;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, PartialEq)]
pub enum WireState {
	Low,
	High,
	Floating,
}

impl Copy for WireState {}

#[derive(Clone, Debug)]
pub struct Wire {
	state: Arc<RwLock<WireState>>,
	pull: WireState,
}

impl Wire {
	pub fn new() -> Self {
		Self::new_with_pull(WireState::Floating)
	}

	pub fn new_with_pull(pull: WireState) -> Self {
		Self {
			state: Arc::new(RwLock::new(WireState::Floating)),
			pull,
		}
	}

	pub fn set_state(&mut self, state: WireState) {
		*self.state.write().unwrap() = state;
	}

	pub fn get_state(&self) -> WireState {
		let s = *self.state.read().unwrap();
		if s == WireState::Floating {
			self.pull
		} else {
			s
		}
	}

	pub fn as_push_pull_pin(&self) -> PushPullPin {
		PushPullPin { wire: self.clone() }
	}

	pub fn as_open_drain_pin(&self) -> OpenDrainPin {
		OpenDrainPin { wire: self.clone() }
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
		self.wire.set_state(WireState::Low);
		Ok(())
	}

	fn try_set_high(&mut self) -> Result<(), Self::Error> {
		self.wire.set_state(WireState::High);
		Ok(())
	}
}

pub struct OpenDrainPin {
	wire: Wire,
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
		self.wire.set_state(WireState::Floating);
		Ok(())
	}

	fn try_set_high(&mut self) -> Result<(), Self::Error> {
		self.wire.set_state(WireState::Low);
		Ok(())
	}
}
