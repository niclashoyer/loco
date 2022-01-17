pub use embedded_hal::timer::nb::{Cancel, CountDown};
pub use embedded_hal_mock::pin;
pub use embedded_time::duration::*;
pub use embedded_time::rate::*;
pub use pin::{Mock, State, Transaction};

pub struct MockTimer {
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

    fn start<T: Into<Nanoseconds<u64>>>(&mut self, timeout: T) -> Result<(), Self::Error> {
        self.count = timeout.into().0 * self.clock.0 / 1_000_000_000_u64;
        Ok(())
    }

    fn wait(&mut self) -> nb::Result<(), Self::Error> {
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
    fn cancel(&mut self) -> Result<(), Self::Error> {
        if self.count > 0 {
            self.count = 0;
            Ok(())
        } else {
            Err(())
        }
    }
}
