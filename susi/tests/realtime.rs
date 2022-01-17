use linux_embedded_hal as hal;
use std::thread;
use thread_priority::*;

use embedded_hal_sync_pins::wire::*;

use drogue_embedded_timer::embedded_countdown;
use embedded_hal::timer::nb::CountDown;
use hal::SysTimer;
use loco_core::drive::Direction;
use loco_susi::message::Msg;

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

fn set_realtime_priority(prio: u32) {
    let thread_id = thread_native_id();
    set_thread_priority_and_policy(
        thread_id,
        ThreadPriority::Specific(prio),
        ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::RoundRobin),
    )
    .unwrap_or_else(|_| {
        eprintln!("WARNING: no realtime scheduling possible, the integration tests might fail!");
    });
}

type SusiWriter = loco_susi::writer::Writer<OpenDrainPin, PushPullPin, UsToStdCountDown<SysTimer>>;
type SusiReader = loco_susi::reader::Reader<OpenDrainPin, InputOnlyPin, MsToStdCountDown<SysTimer>>;

fn write_and_read<FS: 'static, FR: 'static>(write: FS, read: FR) -> Vec<Msg>
where
    FS: FnOnce(SusiWriter) -> () + Send,
    FR: FnOnce(SusiReader) -> Vec<Msg> + Send,
{
    use std::thread::sleep;
    use std::time::Duration;

    let wire_clk = Wire::new();
    let wire_data = Wire::new_with_pull(WireState::High);

    let writer_pin_clk = wire_clk.connect_push_pull_pin();
    let writer_pin_data = wire_data.connect_open_drain_pin();

    let reader_pin_clk = wire_clk.connect_input_pin();
    let reader_pin_data = wire_data.connect_open_drain_pin();

    let writer = thread::spawn(move || {
        set_realtime_priority(80);
        let timer = hal::SysTimer::new();
        let timer = UsToStdCountDown::from(timer);
        let writer = loco_susi::writer::Writer::new(writer_pin_data, writer_pin_clk, timer);
        sleep(Duration::from_millis(200));
        write(writer)
    });
    let reader = thread::spawn(move || {
        set_realtime_priority(90);
        let timer = hal::SysTimer::new();
        let timer = MsToStdCountDown::from(timer);
        let reader = loco_susi::reader::Reader::new(reader_pin_data, reader_pin_clk, timer);
        read(reader)
    });
    let rec = reader.join().unwrap();
    writer.join().unwrap();
    rec
}

fn write_and_read_messages(msgs: Vec<Msg>) {
    use std::thread::sleep;
    use std::time::Duration;

    let mut write_msgs = msgs.clone();
    let num = msgs.len();

    let writer = move |mut writer: SusiWriter| {
        write_msgs.reverse();
        sleep(Duration::from_millis(200));
        while let Some(msg) = write_msgs.pop() {
            loop {
                let res = writer.write(&msg);
                if let Ok(_) = res {
                    break;
                } else if res != Err(nb::Error::WouldBlock) {
                    panic!("{:?}", res);
                }
                let slept = std::time::Instant::now();
                sleep(Duration::from_micros(50));
                let slept = slept.elapsed().as_micros();
                if slept > 200 {
                    eprintln!(
                        "WARNING: writer slept {} µs, more than 200 µs will cause problems",
                        slept
                    );
                }
            }
        }
    };
    let reader = move |mut reader: SusiReader| {
        let start = std::time::Instant::now();
        let mut recv = vec![];
        loop {
            if start.elapsed().as_millis() > 2000 {
                panic!("reader timed out - buf: {:?}", recv);
            }
            let res = reader.read();
            if let Ok(msg) = res {
                recv.push(msg);
                if recv.len() == num {
                    return recv;
                }
            } else if res != Err(nb::Error::WouldBlock) {
                panic!("{:?}", res);
            }
            let slept = std::time::Instant::now();
            sleep(Duration::from_micros(50));
            let slept = slept.elapsed().as_micros();
            if slept > 200 {
                eprintln!(
                    "WARNING: reader slept {} µs, more than 200 µs will cause problems",
                    slept
                );
            }
        }
    };

    let recv = write_and_read(writer, reader);
    assert_eq!(recv, msgs);
}

use serial_test::serial;

#[test]
#[serial]
fn single_message() {
    write_and_read_messages(vec![Msg::LocomotiveSpeed(Direction::Forward, 120)]);
}

#[test]
#[serial]
fn two_messages() {
    write_and_read_messages(vec![
        Msg::LocomotiveSpeed(Direction::Forward, 120),
        Msg::LocomotiveSpeed(Direction::Forward, 120),
    ]);
}

#[test]
#[serial]
fn three_messages() {
    write_and_read_messages(vec![
        Msg::LocomotiveSpeed(Direction::Forward, 120),
        Msg::LocomotiveSpeed(Direction::Forward, 120),
        Msg::LocomotiveSpeed(Direction::Forward, 120),
    ]);
}
