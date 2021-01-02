use linux_embedded_hal as hal;
use std::thread;

mod wire;
use wire::*;

use drogue_embedded_timer::embedded_countdown;
use embedded_hal::timer::CountDown;

embedded_countdown!(
	MsToStdCountDown,
	embedded_time::duration::Milliseconds,
	std::time::Duration
	=> (ms) {
		std::time::Duration::from_millis(ms.0 as u64)
	}
);

embedded_countdown!(
	UsToStdCountDown,
	embedded_time::duration::Microseconds,
	std::time::Duration
	=> (us) {
		std::time::Duration::from_micros(us.0 as u64)
	}
);

#[test]
fn send_and_receive() {
	use std::thread::sleep;
	use std::time::Duration;
	use susi::message::{Direction, Msg};

	let wire_clk = Wire::new();
	let wire_data = Wire::new_with_pull(WireState::High);

	let sender_pin_clk = wire_clk.as_push_pull_pin();
	let sender_pin_data = wire_data.as_open_drain_pin();

	let receiver_pin_clk = wire_clk.as_input_pin();
	let receiver_pin_data = wire_data.as_open_drain_pin();

	let sender = thread::spawn(move || {
		let timer = hal::SysTimer::new();
		let timer = UsToStdCountDown::from(timer);
		let mut sender = susi::sender::Sender::new(sender_pin_data, sender_pin_clk, timer);

		let msg = Msg::LocomotiveSpeed(Direction::Forward, 120);
		println!("sending {:?} as {:?}", msg, msg.to_bytes());

		sleep(Duration::from_millis(200));
		loop {
			let res = sender.write(&msg);
			if let Ok(_) = res {
				return;
			} else if res != Err(nb::Error::WouldBlock) {
				panic!(res);
			}
			sleep(Duration::from_nanos(500));
		}
	});
	let receiver = thread::spawn(move || {
		let timer = hal::SysTimer::new();
		let timer = MsToStdCountDown::from(timer);
		let mut receiver =
			susi::receiver::Receiver::new(receiver_pin_data, receiver_pin_clk, timer);

		let start = std::time::Instant::now();
		loop {
			if start.elapsed().as_millis() > 2000 {
				panic!("receiver timeout");
			}
			let res = receiver.read();
			if let Ok(msg) = res {
				println!("received: {:?}", msg);
				return msg;
			} else if res != Err(nb::Error::WouldBlock) {
				panic!(res);
			}
			sleep(Duration::from_micros(100));
		}
	});

	assert_eq!((), sender.join().unwrap());
	assert_eq!(
		Msg::LocomotiveSpeed(Direction::Forward, 120),
		receiver.join().unwrap()
	);
}
