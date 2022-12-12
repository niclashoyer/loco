use embassy_futures::select::{select, Either};
use embedded_hal_async::delay::DelayUs;
use embedded_hal_async::digital::Wait;

use crate::message::Message;
use crate::Error;

use log::{debug, trace};

const BUF_SIZE: usize = 8;
const TIMEOUT_ONE: u32 = 73;

pub trait Reader {
    async fn read(&mut self) -> Result<Message, Error>;
}

pub struct PinDelayReader<DCC, US> {
    pin_dcc: DCC,
    delay: US,
}

impl<DCC, US> PinDelayReader<DCC, US>
where
    DCC: Wait,
    US: DelayUs,
{
    pub fn new(pin_dcc: DCC, delay: US) -> Self {
        Self { pin_dcc, delay }
    }
}

impl<DCC, US> PinDelayReader<DCC, US>
where
    DCC: Wait,
    US: DelayUs,
{
    async fn read_half_bit(&mut self) -> Result<bool, Error> {
        let result = select(
            self.pin_dcc.wait_for_any_edge(),
            self.delay.delay_us(TIMEOUT_ONE),
        )
        .await;
        if let Either::First(_) = result {
            Ok(true)
        } else {
            self.pin_dcc
                .wait_for_any_edge()
                .await
                .map_err(|_| Error::IOError)?;
            Ok(false)
        }
    }

    async fn read_bit(&mut self) -> Result<bool, Error> {
        let mut last_half = None;
        loop {
            let half = self.read_half_bit().await?;
            trace!("read edge as {}", half as i32);
            if let Some(other) = last_half {
                if other == half {
                    trace!("read bit {}", half as i32);
                    return Ok(half);
                }
            }
            last_half = Some(half);
        }
    }

    async fn read_preamble(&mut self) -> Result<(), Error> {
        let mut ones = 0;
        loop {
            let bit = self.read_bit().await?;
            if bit {
                ones += 1;
            } else if ones > 9 {
                return Ok(());
            } else {
                ones = 0;
            }
        }
    }

    async fn read_byte(&mut self) -> Result<u8, Error> {
        let mut byte = 0x00;
        for i in 0..7 {
            let bit = self.read_bit().await?;
            if bit {
                byte |= 1 << i;
            }
        }
        Ok(byte)
    }
}

impl<DCC, US> Reader for PinDelayReader<DCC, US>
where
    DCC: Wait,
    US: DelayUs,
{
    async fn read(&mut self) -> Result<Message, Error> {
        let mut buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
        let mut current_byte = 0;

        self.read_preamble().await?;
        debug!("detected preamble + zero, start reading bits");
        while current_byte < BUF_SIZE {
            buf[current_byte] = self.read_byte().await?;
            debug!("read byte {:#04x}", buf[current_byte]);
            if self.read_bit().await? {
                let msg = Message::from_bytes(&buf[..current_byte]);
                debug!("read bytes {:#04X?} as {:?}", &buf[..current_byte], msg);
                return Ok(msg);
            }
            current_byte += 1;
        }
        Err(Error::OverflowError)
    }
}
