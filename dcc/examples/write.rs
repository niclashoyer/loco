use embedded_hal_mock_clock::*;
use embedded_hal_vcd::writer::VcdWriterBuilder;
use embedded_time::duration::*;
use loco_core::{
    address::Address,
    drive::{Direction, Speed},
};
use loco_dcc::{
    message::Message,
    writer::{PinEncoder, Writer},
};

use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), std::io::Error> {
    env_logger::init();
    // construct a clock used for simulation
    let mut clock = SimClock::new();
    let writer_timer = clock.get_timer();
    // construct a vcd reader
    let f = BufWriter::new(File::create("examples/write.vcd")?);
    let mut builder = VcdWriterBuilder::new(f).unwrap();
    let out_pin = builder.add_push_pull_pin("dcc").unwrap();
    let mut vcd_writer = builder.build().unwrap();
    // construct dcc writer using pin and timer
    let encoder = PinEncoder::new(out_pin, writer_timer);
    let mut dcc_writer = Writer::new(encoder);

    let timeout = 500_u32.milliseconds();

    let addr = Address { num: 23 };
    let msg = Message::Drive(addr, Direction::Forward, Speed::Steps28(14));
    let _ = dcc_writer.write(&msg);

    loop {
        if clock.elapsed() > timeout {
            break;
        }
        vcd_writer.timestamp(clock.elapsed()).unwrap();
        vcd_writer.sample().unwrap();
        let _ = dcc_writer.write(&msg);
        clock.tick(500_u64.nanoseconds());
    }

    Ok(())
}
