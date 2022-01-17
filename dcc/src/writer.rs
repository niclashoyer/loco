use embedded_hal::digital::blocking::ToggleableOutputPin;
use embedded_hal::timer::nb::CountDown;
use embedded_time::duration::*;

use crate::message::Message;
use crate::Error;

use log::{debug, trace};

const BUF_SIZE: usize = 8;
const PREAMBLE_SIZE: u8 = 14;
// half bit lengths in microseconds
const ONE_HALF_BIT: u32 = 58;
const ZERO_HALF_BIT: u32 = 100;

pub trait Encoder {
    fn write(&mut self, bit: &Bit) -> nb::Result<(), Error>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Bit {
    One,
    Zero,
}

impl Copy for Bit {}

#[derive(Debug, PartialEq)]
enum EncoderState {
    Idle,
    WritingFirstHalf,
    WritingSecondHalf,
}

#[derive(Debug)]
pub struct PinEncoder<DCC, TIM> {
    pin_dcc: DCC,
    timer: TIM,
    state: EncoderState,
}

impl<DCC, TIM> PinEncoder<DCC, TIM>
where
    DCC: ToggleableOutputPin,
    TIM: CountDown,
    TIM::Time: From<Microseconds<u32>>,
{
    pub fn new(pin_dcc: DCC, timer: TIM) -> Self {
        Self {
            pin_dcc,
            timer,
            state: EncoderState::Idle,
        }
    }

    fn toggle_pin(&mut self) -> Result<(), Error> {
        self.pin_dcc.toggle().map_err(|_| Error::IOError)
    }

    fn start_timer(&mut self, bit: &Bit) -> Result<(), Error> {
        let d = match bit {
            Bit::One => ONE_HALF_BIT,
            Bit::Zero => ZERO_HALF_BIT,
        };
        self.timer
            .start(d.microseconds())
            .map_err(|_| Error::TimerError)
    }

    fn wait_timer(&mut self) -> nb::Result<(), Error> {
        self.timer.wait().map_err(|e| e.map(|_| Error::TimerError))
    }
}

impl<DCC, TIM> Encoder for PinEncoder<DCC, TIM>
where
    DCC: ToggleableOutputPin,
    TIM: CountDown,
    TIM::Time: From<Microseconds<u32>>,
{
    #[inline]
    fn write(&mut self, bit: &Bit) -> nb::Result<(), Error> {
        use EncoderState::*;
        if self.state != Idle {
            self.wait_timer()?;
        }
        match self.state {
            Idle => {
                self.start_timer(bit)?;
                self.state = WritingFirstHalf;
            }
            WritingFirstHalf => {
                self.toggle_pin()?;
                self.start_timer(bit)?;
                self.state = WritingSecondHalf;
            }
            WritingSecondHalf => {
                self.toggle_pin()?;
                self.state = Idle;
                return Ok(());
            }
        }
        Err(nb::Error::WouldBlock)
    }
}

#[derive(Debug, PartialEq)]
enum State {
    Idle,
    Preamble(u8),
    Zero,
    Writing(Bit),
    End,
}

pub struct Writer<E> {
    encoder: E,
    state: State,
    buf: [u8; BUF_SIZE],
    bytes_to_write: usize,
    bits_written: usize,
}

impl<E> Writer<E>
where
    E: Encoder,
{
    #[inline]
    pub fn new(encoder: E) -> Self {
        Self {
            encoder,
            state: State::Idle,
            buf: [0; BUF_SIZE],
            bytes_to_write: 0,
            bits_written: 0,
        }
    }

    #[inline]
    fn write_preamble(&mut self, left: u8) -> nb::Result<(), Error> {
        use State::*;
        if self.state != Preamble(left) {
            self.state = Preamble(left);
        }
        self.encoder.write(&Bit::One)
    }

    #[inline]
    fn write_zero(&mut self) -> nb::Result<(), Error> {
        use State::*;
        if self.state != Zero {
            self.state = Zero;
        }
        self.encoder.write(&Bit::Zero)
    }

    #[inline]
    fn next_bit(&mut self) -> Bit {
        let num = self.bits_written / 8;
        if (self.buf[num] >> (7 - (self.bits_written % 8))) & 0x01 == 0x01 {
            Bit::One
        } else {
            Bit::Zero
        }
    }

    #[inline]
    fn write_bit(&mut self, bit: &Bit) -> nb::Result<(), Error> {
        use State::*;
        if self.state != Writing(*bit) {
            self.state = Writing(*bit);
        }
        self.state = Writing(*bit);
        self.encoder.write(&bit)?;
        self.bits_written += 1;
        Ok(())
    }

    #[inline]
    fn write_end(&mut self) -> nb::Result<(), Error> {
        use State::*;
        if self.state != End {
            self.state = End;
        }
        self.encoder.write(&Bit::One)
    }

    pub fn write(&mut self, msg: &Message) -> nb::Result<(), Error> {
        use State::*;
        trace!(
            "{:<20}{:<20}{:<20}",
            format!("{:?}", self.state),
            "",
            format!("{}/{}", self.bits_written, self.bytes_to_write * 8)
        );
        match self.state {
            Idle => {
                self.bytes_to_write = msg.to_buf(&mut self.buf);
                debug!(
                    "writing {:?} as {:#04X?}",
                    msg,
                    &self.buf[..self.bytes_to_write]
                );
                self.bits_written = 0;
                debug!("starting preamble");
                self.write_preamble(PREAMBLE_SIZE)
            }
            Preamble(left) => {
                self.write_preamble(left)?;
                if left > 0 {
                    self.write_preamble(left - 1)
                } else {
                    debug!("preamble done, writing zero");
                    self.write_zero()
                }
            }
            Zero => {
                self.write_zero()?;
                debug!("zero done, starting next byte");
                let bit = self.next_bit();
                trace!("next bit: {:?}", bit);
                self.write_bit(&bit)
            }
            Writing(bit) => {
                self.write_bit(&bit)?;
                if self.bits_written == self.bytes_to_write * 8 {
                    debug!(
                        "byte {:#04x} done, message written, writing one",
                        self.buf[(self.bits_written - 1) / 8]
                    );
                    self.write_end()
                } else if self.bits_written % 8 == 0 {
                    debug!(
                        "byte {:#04x} done, writing zero",
                        self.buf[(self.bits_written - 1) / 8]
                    );
                    self.write_zero()
                } else {
                    let bit = self.next_bit();
                    trace!("next bit: {:?}", bit);
                    self.write_bit(&bit)
                }
            }
            End => {
                self.write_end()?;
                debug!("finished");
                self.state = Idle;
                return Ok(());
            }
        }
    }
}
