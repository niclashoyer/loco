mod time;
use embedded_time::duration::*;
use time::*;

use embedded_hal_sync_pins::wire::*;

use loco_core::drive::Direction;
use loco_susi::message::Msg;

type SusiWriter = loco_susi::writer::Writer<OpenDrainPin, PushPullPin, SimTimer>;
type SusiReader = loco_susi::reader::Reader<OpenDrainPin, InputOnlyPin, SimTimer>;

enum Error {}

fn write_and_read<FS: 'static, FR: 'static>(
	mut write: FS,
	mut read: FR,
	timeout: u32,
) -> Vec<Msg>
where
	FS: FnMut(&mut SusiWriter, &SimClock) -> nb::Result<(), Error>,
	FR: FnMut(&mut SusiReader, &SimClock) -> nb::Result<Vec<Msg>, Error>,
{
	let wire_clk = Wire::new();
	let wire_data = Wire::new_with_pull(WireState::High);

	let writer_pin_clk = wire_clk.as_push_pull_pin();
	let writer_pin_data = wire_data.as_open_drain_pin();

	let reader_pin_clk = wire_clk.as_input_pin();
	let reader_pin_data = wire_data.as_open_drain_pin();

	let mut clock = SimClock::new();
	let writer_timer = clock.get_timer();
	let reader_timer = clock.get_timer();

	let mut writer = loco_susi::writer::Writer::new(writer_pin_data, writer_pin_clk, writer_timer);
	let mut reader =
		loco_susi::reader::Reader::new(reader_pin_data, reader_pin_clk, reader_timer);

	let mut recv = vec![];
	let mut writer_done = false;
	let mut reader_done = false;

	loop {
		if clock.elapsed() > timeout.milliseconds() {
			panic!("simulation timed out");
		}
		if !writer_done {
			if let Ok(_) = write(&mut writer, &clock) {
				writer_done = true;
				println!("writer done");
			}
		}
		if !reader_done {
			if let Ok(msgs) = read(&mut reader, &clock) {
				recv = msgs;
				reader_done = true;
				println!("reader done");
			}
		}
		if writer_done && reader_done {
			break;
		}
		clock.tick(100_u64.nanoseconds());
	}
	recv
}

fn write_and_read_messages(msgs: Vec<Msg>) {
	let mut write_msgs = msgs.clone();
	write_msgs.reverse();
	let num = msgs.len();
	let mut msg = write_msgs.pop().expect("at least one message must be sent");
	let mut recv = vec![];

	let writer = move |writer: &mut SusiWriter, _clock: &SimClock| {
		let res = writer.write(&msg);
		if let Ok(_) = res {
			if write_msgs.is_empty() {
				return Ok(());
			} else {
				msg = write_msgs.pop().unwrap();
			}
		} else if res != Err(nb::Error::WouldBlock) {
			panic!(res);
		}
		Err(nb::Error::WouldBlock)
	};
	let reader = move |reader: &mut SusiReader, _clock: &SimClock| {
		let res = reader.read();
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

	let recv = write_and_read(writer, reader, 500);
	assert_eq!(recv, msgs);
}

#[test]
fn single_message() {
	write_and_read_messages(vec![Msg::LocomotiveSpeed(Direction::Forward, 120)]);
}

#[test]
fn two_messages() {
	write_and_read_messages(vec![
		Msg::LocomotiveSpeed(Direction::Forward, 120),
		Msg::LocomotiveSpeed(Direction::Forward, 120),
	]);
}

#[test]
fn three_messages() {
	write_and_read_messages(vec![
		Msg::LocomotiveSpeed(Direction::Forward, 120),
		Msg::LocomotiveSpeed(Direction::Forward, 120),
		Msg::LocomotiveSpeed(Direction::Forward, 120),
	]);
}

#[test]
fn timing_issues() {
	let mut write_msgs = vec![
		Msg::LocomotiveSpeed(Direction::Forward, 10),
		Msg::LocomotiveSpeed(Direction::Forward, 20),
		Msg::LocomotiveSpeed(Direction::Forward, 30),
	];
	write_msgs.reverse();
	let mut msg = write_msgs.pop().unwrap();
	let mut recv = vec![];
	let mut shift = None;
	let mut reset = None;

	let writer = move |writer: &mut SusiWriter, clock: &SimClock| {
		if reset.is_none() {
			if let Some(s) = shift {
				// write for ~2ms, then reset (corrupting the second message)
				if (clock.elapsed() - s) > 2_u32.milliseconds() {
					reset = Some(clock.elapsed());
					shift = None;
					return Err(nb::Error::WouldBlock);
				}
			}
			let res = writer.write(&msg);
			if let Ok(_) = res {
				if write_msgs.is_empty() {
					return Ok(());
				} else {
					msg = write_msgs.pop().unwrap();
					if write_msgs.len() == 1 {
						// one message left, lets corrupt the timing while writeing
						shift = Some(clock.elapsed());
					}
					if write_msgs.len() == 0 {
						// no message left, reset now to get the last message right again
						reset = Some(clock.elapsed());
					}
				}
			} else if res != Err(nb::Error::WouldBlock) {
				panic!(res);
			}
		} else {
			// wait at least 10 ms to reset the reader
			if (clock.elapsed() - reset.unwrap()) > 10_u32.milliseconds() {
				reset = None;
			}
		}
		Err(nb::Error::WouldBlock)
	};
	let reader = move |reader: &mut SusiReader, _clock: &SimClock| {
		let res = reader.read();
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

	let recv = write_and_read(writer, reader, 500);
	assert_eq!(
		recv,
		vec![
			Msg::LocomotiveSpeed(Direction::Forward, 10),
			Msg::LocomotiveSpeed(Direction::Forward, 30),
		]
	);
}
