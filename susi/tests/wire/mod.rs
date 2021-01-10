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
