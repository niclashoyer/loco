use embedded_hal_mock_clock::*;
use embedded_hal_sync_pins::wire::*;
use embedded_time::duration::*;
use log::trace;
use test_log::test;

use loco_core::{
    address::Address,
    drive::{Direction, Speed},
};
use loco_dcc::{
    message::Message,
    reader::{PinDecoder, Reader},
    writer::{PinEncoder, Writer},
};

type DccWriter = Writer<PinEncoder<PushPullPin, SimTimer>>;
type DccReader = Reader<PinDecoder<InputOnlyPin, SimTimer>>;

enum Error {}

fn write_and_read<FS: 'static, FR: 'static>(
    mut write: FS,
    mut read: FR,
    timeout: u32,
) -> Vec<Message>
where
    FS: FnMut(&mut DccWriter, &SimClock) -> nb::Result<(), Error>,
    FR: FnMut(&mut DccReader, &SimClock) -> nb::Result<Vec<Message>, Error>,
{
    let wire_dcc = Wire::new_with_pull(WireState::High);

    let writer_pin_dcc = wire_dcc.connect_push_pull_pin();
    let reader_pin_dcc = wire_dcc.connect_input_pin();

    let mut clock = SimClock::new();
    let writer_timer = clock.get_timer();
    let reader_timer = clock.get_timer();

    let encoder = loco_dcc::writer::PinEncoder::new(writer_pin_dcc, writer_timer);
    let mut writer = loco_dcc::writer::Writer::new(encoder);
    let mut reader = loco_dcc::reader::Reader::new(PinDecoder::new(reader_pin_dcc, reader_timer));

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
        if !writer_done {
            if clock.elapsed().0 % 10_000 == 0 {
                trace!("clock: {}µs", clock.elapsed() / 1000);
            }
        }
        clock.tick(1000_u64.nanoseconds());
    }
    recv
}

fn write_and_read_messages(msgs: Vec<Message>) {
    let mut write_msgs = msgs.clone();
    write_msgs.reverse();
    let num = msgs.len();
    let mut msg = write_msgs.pop().expect("at least one message must be sent");
    let mut recv = vec![];

    let writer = move |writer: &mut DccWriter, _clock: &SimClock| {
        let res = writer.write(&msg);
        if let Ok(_) = res {
            if write_msgs.is_empty() {
                return Ok(());
            } else {
                msg = write_msgs.pop().unwrap();
            }
        } else if res != Err(nb::Error::WouldBlock) {
            panic!("{:?}", res);
        }
        Err(nb::Error::WouldBlock)
    };
    let reader = move |reader: &mut DccReader, _clock: &SimClock| {
        let res = reader.read();
        if let Ok(msg) = res {
            recv.push(msg);
            if recv.len() == num {
                return Ok(recv.clone());
            }
        } else if res != Err(nb::Error::WouldBlock) {
            panic!("{:?}", res);
        }
        Err(nb::Error::WouldBlock)
    };

    let recv = write_and_read(writer, reader, 500);
    assert_eq!(recv, msgs);
}

#[test]
fn single_message() {
    write_and_read_messages(vec![
        Message::Drive(Address { num: 23 }, Direction::Forward, Speed::Steps128(56)),
        Message::Drive(Address { num: 2 }, Direction::Forward, Speed::Steps128(4)),
    ]);
}
