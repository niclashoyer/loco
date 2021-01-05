pub use crate::wire::*;

use std::convert::Infallible;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use embedded_hal::timer::{CountDown, Periodic};
use embedded_time::{clock, duration::*, fraction::Fraction, Clock, Instant};

#[derive(Clone, Debug)]
pub struct SimClock {
	ticks: Arc<AtomicU64>,
}

impl Clock for SimClock {
	type T = u64;
	const SCALING_FACTOR: Fraction = Fraction::new(1_000_000_000, 1);

	fn try_now(&self) -> Result<Instant<Self>, clock::Error> {
		let ticks: u64 = self.ticks.load(Ordering::Relaxed);
		Ok(Instant::<Self>::new(ticks))
	}
}

impl SimClock {
	pub fn tick(&mut self) {
		self.ticks.fetch_add(1, Ordering::Relaxed);
	}

	pub fn get_timer<D>(&self, duration: D) -> SimTimer
	where
		D: Into<Nanoseconds>,
	{
		let clock = self.clone();
		let duration = duration.into();
		let expiration = clock.try_now().unwrap() + duration;
		SimTimer {
			clock: self.clone(),
			duration,
			expiration,
		}
	}
}

pub struct SimTimer {
	clock: SimClock,
	duration: Nanoseconds,
	expiration: Instant<SimClock>,
}

impl CountDown for SimTimer {
	type Error = Infallible;
	type Time = Nanoseconds;

	fn try_start<T>(&mut self, count: T) -> Result<(), Self::Error>
	where
		T: Into<Self::Time>,
	{
		self.duration = count.into();
		Ok(())
	}

	fn try_wait(&mut self) -> nb::Result<(), Self::Error> {
		let now = self.clock.try_now().unwrap();
		if now >= self.expiration {
			self.expiration = now + self.duration;
			Ok(())
		} else {
			Err(nb::Error::WouldBlock)
		}
	}
}

impl Periodic for SimTimer {}

pub struct Simulation {
	clock: SimClock,
}
