mod time;
use embedded_time::duration::*;
use time::*;

use embedded_hal_sync_pins::wire::*;

use loco_core::drive::Direction;
use loco_susi::message::Msg;

type SusiSender = loco_susi::sender::Sender<OpenDrainPin, PushPullPin, SimTimer>;
type SusiReceiver = loco_susi::receiver::Receiver<OpenDrainPin, InputOnlyPin, SimTimer>;

enum Error {}

fn send_and_receive<FS: 'static, FR: 'static>(
	mut send: FS,
	mut receive: FR,
	timeout: u32,
) -> Vec<Msg>
where
	FS: FnMut(&mut SusiSender, &SimClock) -> nb::Result<(), Error>,
	FR: FnMut(&mut SusiReceiver, &SimClock) -> nb::Result<Vec<Msg>, Error>,
{
	let wire_clk = Wire::new();
	let wire_data = Wire::new_with_pull(WireState::High);

	let sender_pin_clk = wire_clk.as_push_pull_pin();
	let sender_pin_data = wire_data.as_open_drain_pin();

	let receiver_pin_clk = wire_clk.as_input_pin();
	let receiver_pin_data = wire_data.as_open_drain_pin();

	let mut clock = SimClock::new();
	let sender_timer = clock.get_timer();
	let receiver_timer = clock.get_timer();

	let mut sender = loco_susi::sender::Sender::new(sender_pin_data, sender_pin_clk, sender_timer);
	let mut receiver =
		loco_susi::receiver::Receiver::new(receiver_pin_data, receiver_pin_clk, receiver_timer);

	let mut recv = vec![];
	let mut sender_done = false;
	let mut receiver_done = false;

	loop {
		if clock.elapsed() > timeout.milliseconds() {
			panic!("simulation timed out");
		}
		if !sender_done {
			if let Ok(_) = send(&mut sender, &clock) {
				sender_done = true;
				println!("sender done");
			}
		}
		if !receiver_done {
			if let Ok(msgs) = receive(&mut receiver, &clock) {
				recv = msgs;
				receiver_done = true;
				println!("receiver done");
			}
		}
		if sender_done && receiver_done {
			break;
		}
		clock.tick(100_u64.nanoseconds());
	}
	recv
}

fn send_and_receive_messages(msgs: Vec<Msg>) {
	let mut send_msgs = msgs.clone();
	send_msgs.reverse();
	let num = msgs.len();
	let mut msg = send_msgs.pop().expect("at least one message must be sent");
	let mut recv = vec![];

	let sender = move |sender: &mut SusiSender, _clock: &SimClock| {
		let res = sender.write(&msg);
		if let Ok(_) = res {
			if send_msgs.is_empty() {
				return Ok(());
			} else {
				msg = send_msgs.pop().unwrap();
			}
		} else if res != Err(nb::Error::WouldBlock) {
			panic!(res);
		}
		Err(nb::Error::WouldBlock)
	};
	let receiver = move |receiver: &mut SusiReceiver, _clock: &SimClock| {
		let res = receiver.read();
		if let Ok(msg) = res {
			recv.push(msg);
			if recv.len() == num {
				return Ok(recv.clone());
			}
		} else if res != Err(nb::Error::WouldBlock) {
			panic!(res);
		}
		Err(nb::Error::WouldBlock)
	};

	let recv = send_and_receive(sender, receiver, 500);
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
		Msg::LocomotiveSpeed(Direction::Forward, 120),
	]);
}

#[test]
fn three_messages() {
	send_and_receive_messages(vec![
		Msg::LocomotiveSpeed(Direction::Forward, 120),
		Msg::LocomotiveSpeed(Direction::Forward, 120),
		Msg::LocomotiveSpeed(Direction::Forward, 120),
	]);
}

#[test]
fn timing_issues() {
	let mut send_msgs = vec![
		Msg::LocomotiveSpeed(Direction::Forward, 10),
		Msg::LocomotiveSpeed(Direction::Forward, 20),
		Msg::LocomotiveSpeed(Direction::Forward, 30),
	];
	send_msgs.reverse();
	let mut msg = send_msgs.pop().unwrap();
	let mut recv = vec![];
	let mut shift = None;
	let mut reset = None;

	let sender = move |sender: &mut SusiSender, clock: &SimClock| {
		if reset.is_none() {
			if let Some(s) = shift {
				// send for ~2ms, then reset (corrupting the second message)
				if (clock.elapsed() - s) > 2_u32.milliseconds() {
					reset = Some(clock.elapsed());
					shift = None;
					return Err(nb::Error::WouldBlock);
				}
			}
			let res = sender.write(&msg);
			if let Ok(_) = res {
				if send_msgs.is_empty() {
					return Ok(());
				} else {
					msg = send_msgs.pop().unwrap();
					if send_msgs.len() == 1 {
						// one message left, lets corrupt the timing while sending
						shift = Some(clock.elapsed());
					}
					if send_msgs.len() == 0 {
						// no message left, reset now to get the last message right again
						reset = Some(clock.elapsed());
					}
				}
			} else if res != Err(nb::Error::WouldBlock) {
				panic!(res);
			}
		} else {
			// wait at least 10 ms to reset the receiver
			if (clock.elapsed() - reset.unwrap()) > 10_u32.milliseconds() {
				reset = None;
			}
		}
		Err(nb::Error::WouldBlock)
	};
	let receiver = move |receiver: &mut SusiReceiver, _clock: &SimClock| {
		let res = receiver.read();
		if let Ok(msg) = res {
			recv.push(msg);
			if recv.len() == 2 {
				return Ok(recv.clone());
			}
		} else if res != Err(nb::Error::WouldBlock) {
			panic!(res);
		}
		Err(nb::Error::WouldBlock)
	};

	let recv = send_and_receive(sender, receiver, 500);
	assert_eq!(
		recv,
		vec![
			Msg::LocomotiveSpeed(Direction::Forward, 10),
			Msg::LocomotiveSpeed(Direction::Forward, 30),
		]
	);
}
