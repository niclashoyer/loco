use drogue_embedded_timer::embedded_countdown;
use embedded_hal::timer::nb::CountDown;
use linux_embedded_hal::{
    gpio_cdev::{Chip, LineRequestFlags},
    CdevPin, SysTimer,
};
use loco_command_station::*;
use loco_core::address::Address;
use loco_core::drive::{Direction, Speed};
use loco_dcc::writer::PinEncoder;
use log::trace;
use nb::block;
use std::io::{stdout, Read, Write};
use termion::async_stdin;

embedded_countdown!(
    UsToStdCountDown,
    embedded_time::duration::Microseconds,
    std::time::Duration
    => (us) {
        std::time::Duration::from_micros(us.0 as u64)
    }
);

fn main() {
    env_logger::init();

    let mut stdout = stdout();
    //let mut stdout = stdout.lock().into_raw_mode().unwrap();
    let mut stdin = async_stdin().bytes();

    let mut chip = Chip::new("/dev/gpiochip0").unwrap();
    let handle1 = chip
        .get_line(5)
        .unwrap()
        .request(LineRequestFlags::OUTPUT, 0, "command-station-dcc1")
        .unwrap();
    let handle2 = chip
        .get_line(6)
        .unwrap()
        .request(LineRequestFlags::OUTPUT, 0, "command-station-dcc2")
        .unwrap();
    let pin1 = CdevPin::new(handle1).unwrap();
    let pin2 = CdevPin::new(handle2).unwrap();
    let dcc_pin = togglepins::TogglePins::new(pin1, pin2);
    let timer = SysTimer::new();
    let timer = UsToStdCountDown::from(timer);
    let encoder = PinEncoder::new(dcc_pin, timer);
    let mut station: Station<_, 32> = Station::new(encoder);

    let addr: Address = 3.into();
    let mut speed = 0_i8;

    station.add_loco(3.into());

    loop {
        block!(station.run()).unwrap();
        let b = stdin.next();
        trace!("{:?}", b);
        if let Some(b) = b {
            match b.unwrap() {
                b'A' => {
                    speed += 1;
                }
                b'B' => speed -= 1,
                b'q' => break,
                _ => {}
            }
            write!(
                stdout,
                "{}{}speed: {}",
                termion::cursor::Goto(1, 1),
                termion::clear::CurrentLine,
                speed
            )
            .unwrap();
            stdout.flush().unwrap();
            let dir = if speed >= 0 {
                Direction::Forward
            } else {
                Direction::Backward
            };
            let spd = if speed == 0 {
                Speed::Stop
            } else {
                Speed::Steps128(speed.abs() as u8)
            };

            station.loco_set_drive(addr, spd, dir);
        }
    }
}
