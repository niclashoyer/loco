use std::convert::Infallible;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use embedded_hal::timer::{Cancel, CountDown, Periodic};
pub use embedded_time::Clock;
use embedded_time::{clock, duration::*, fraction::Fraction, Instant};

#[derive(Clone, Debug)]
pub struct SimClock {
	ticks: Arc<AtomicU64>,
}

impl Clock for SimClock {
	type T = u64;
	const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000_000);

	fn try_now(&self) -> Result<Instant<Self>, clock::Error> {
		let ticks: u64 = self.ticks.load(Ordering::Relaxed);
		Ok(Instant::<Self>::new(ticks))
	}
}

impl SimClock {
	pub fn new() -> Self {
		SimClock {
			ticks: Arc::new(AtomicU64::new(0)),
		}
	}

	pub fn elapsed(&self) -> Nanoseconds<u64> {
		self.ticks.load(Ordering::Relaxed).nanoseconds()
	}

	pub fn tick<T>(&mut self, ticks: T)
	where
		T: Into<Nanoseconds<u64>>,
	{
		self.ticks.fetch_add(ticks.into().0, Ordering::Relaxed);
	}

	pub fn get_timer(&self) -> SimTimer {
		let clock = self.clone();
		let duration = 1.nanoseconds();
		let expiration = clock.try_now().unwrap();
		SimTimer {
			clock: self.clone(),
			duration,
			expiration,
			started: false,
		}
	}
}

pub struct SimTimer {
	clock: SimClock,
	duration: Nanoseconds<u64>,
	expiration: Instant<SimClock>,
	started: bool,
}

impl CountDown for SimTimer {
	type Error = Infallible;
	type Time = Nanoseconds<u64>;

	fn try_start<T>(&mut self, count: T) -> Result<(), Self::Error>
	where
		T: Into<Self::Time>,
	{
		let now = self.clock.try_now().unwrap();
		self.duration = count.into();
		self.expiration = now + self.duration;
		self.started = true;
		Ok(())
	}

	fn try_wait(&mut self) -> nb::Result<(), Self::Error> {
		let now = self.clock.try_now().unwrap();
		if self.started && now >= self.expiration {
			self.expiration = now + self.duration;
			Ok(())
		} else {
			Err(nb::Error::WouldBlock)
		}
	}
}

impl Periodic for SimTimer {}

impl Cancel for SimTimer {
	fn try_cancel(&mut self) -> Result<(), Self::Error> {
		self.started = false;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn count_down() {
		let mut clock = SimClock::new();
		let mut timer = clock.get_timer();
		timer.try_start(100_u64.nanoseconds()).unwrap();
		clock.tick(50_u64.nanoseconds());
		assert_eq!(timer.try_wait(), Err(nb::Error::WouldBlock));
		clock.tick(50_u64.nanoseconds());
		assert_eq!(timer.try_wait(), Ok(()));
	}
}
