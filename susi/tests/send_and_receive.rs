use linux_embedded_hal as hal;
use std::thread;

mod wire;
use wire::*;

use drogue_embedded_timer::embedded_countdown;
use embedded_hal::timer::CountDown;
use hal::SysTimer;
use susi::message::{Direction, Msg};

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

type SusiSender = susi::sender::Sender<OpenDrainPin, PushPullPin, UsToStdCountDown<SysTimer>>;
type SusiReceiver =
	susi::receiver::Receiver<OpenDrainPin, InputOnlyPin, MsToStdCountDown<SysTimer>>;

fn send_and_receive<FS: 'static, FR: 'static>(send: FS, receive: FR) -> Vec<Msg>
where
	FS: FnOnce(SusiSender) -> () + Send,
	FR: FnOnce(SusiReceiver) -> Vec<Msg> + Send,
{
	use std::thread::sleep;
	use std::time::Duration;

	let wire_clk = Wire::new();
	let wire_data = Wire::new_with_pull(WireState::High);

	let sender_pin_clk = wire_clk.as_push_pull_pin();
	let sender_pin_data = wire_data.as_open_drain_pin();

	let receiver_pin_clk = wire_clk.as_input_pin();
	let receiver_pin_data = wire_data.as_open_drain_pin();

	let sender = thread::spawn(move || {
		let timer = hal::SysTimer::new();
		let timer = UsToStdCountDown::from(timer);
		let sender = susi::sender::Sender::new(sender_pin_data, sender_pin_clk, timer);

		sleep(Duration::from_millis(200));
		send(sender)
	});
	let receiver = thread::spawn(move || {
		let timer = hal::SysTimer::new();
		let timer = MsToStdCountDown::from(timer);
		let receiver = susi::receiver::Receiver::new(receiver_pin_data, receiver_pin_clk, timer);
		receive(receiver)
	});
	sender.join().unwrap();
	receiver.join().unwrap()
}

fn send_and_receive_messages(msgs: Vec<Msg>) {
	use std::thread::sleep;
	use std::time::Duration;

	let mut send_msgs = msgs.clone();
	let num = msgs.len();

	let sender = move |mut sender: SusiSender| {
		send_msgs.reverse();
		sleep(Duration::from_millis(200));
		while let Some(msg) = send_msgs.pop() {
			println!("sending {:?} as {:?}", msg, msg.to_bytes());
			loop {
				let res = sender.write(&msg);
				if let Ok(_) = res {
					break;
				} else if res != Err(nb::Error::WouldBlock) {
					panic!(res);
				}
			}
		}
	};
	let receiver = move |mut receiver: SusiReceiver| {
		let start = std::time::Instant::now();
		let mut recv = vec![];
		loop {
			if start.elapsed().as_millis() > 2000 {
				panic!("receiver timed out");
			}
			let res = receiver.read();
			if let Ok(msg) = res {
				recv.push(msg);
				if recv.len() == num {
					return recv;
				}
			} else if res != Err(nb::Error::WouldBlock) {
				panic!(res);
			}
		}
	};

	let recv = send_and_receive(sender, receiver);
	assert_eq!(recv, msgs);
}

#[test]
fn single_message() {
	send_and_receive_messages(vec![Msg::LocomotiveSpeed(Direction::Forward, 120)]);
}

#[test]
fn two_messages() {
	send_and_receive_messages(vec![
		Msg::LocomotiveSpeed(Direction::Forward, 120),
		Msg::LocomotiveSpeed(Direction::Backward, 20),
	]);
}
