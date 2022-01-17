use core::convert::Infallible;
use embedded_hal::digital::blocking::{OutputPin, ToggleableOutputPin};

pub struct TogglePins<O1, O2>
where
	O1: OutputPin,
	O2: OutputPin,
{
	pin1: O1,
	pin2: O2,
	state: bool,
}

impl<O1, O2> TogglePins<O1, O2>
where
	O1: OutputPin,
	O2: OutputPin,
{
	pub fn new(mut pin1: O1, mut pin2: O2) -> Self {
		pin1.set_high().unwrap();
		pin2.set_low().unwrap();
		Self {
			pin1,
			pin2,
			state: true,
		}
	}
}

impl<O1, O2> ToggleableOutputPin for TogglePins<O1, O2>
where
	O1: OutputPin,
	O2: OutputPin,
{
	// TODO: remove unwrap, add proper error type
	type Error = Infallible;

	fn toggle(&mut self) -> Result<(), Self::Error> {
		if self.state {
			self.pin1.set_low().unwrap();
			self.pin2.set_high().unwrap();
			self.state = false;
		} else {
			self.pin1.set_high().unwrap();
			self.pin2.set_low().unwrap();
			self.state = true;
		};
		Ok(())
	}
}
