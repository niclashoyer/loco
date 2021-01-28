use embedded_hal_mock_clock::*;
use embedded_hal_vcd::reader::VcdReader;
use embedded_time::duration::*;
use loco_dcc::{message::Message, reader::Reader};

use std::convert::TryInto;
use std::fs::File;
use std::io::BufReader;

#[test]
fn read() -> Result<(), std::io::Error> {
	// construct a clock used for simulation
	let mut clock = SimClock::new();
	let reader_timer = clock.get_timer();
	// construct a vcd reader
	let f = BufReader::new(File::open("tests/fixtures/dcc.vcd")?);
	let mut vcd_reader = VcdReader::new(f).unwrap();
	let in_pin = vcd_reader.get_pin(&["libsigrok", "data"]).unwrap();
	// construct dcc reader using pin and timer
	let mut dcc_reader = Reader::new(in_pin, reader_timer);

	let timeout = 500_u32.milliseconds();
	let mut next_event = 0_u32.nanoseconds();

	loop {
		if clock.elapsed() > timeout {
			panic!("simulation timed out");
		}
		if clock.elapsed() >= next_event {
			next_event = vcd_reader.next().unwrap().try_into().unwrap();
		}
		if let Ok(msg) = dcc_reader.read() {
			println!("read: {:?}", msg);
		}
		clock.tick(500_u64.nanoseconds());
	}

	Ok(())
}
