use embedded_hal::digital::ToggleableOutputPin;
use embedded_hal_async::delay::DelayUs;

use crate::message::Message;
use crate::Error;

use log::{debug, trace};

const BUF_SIZE: usize = 8;
const PREAMBLE_SIZE: u8 = 14;
// half bit lengths in microseconds
const ONE_HALF_BIT: u32 = 58;
const ZERO_HALF_BIT: u32 = 100;

pub trait Writer {
    async fn write<'a>(&'a mut self, msg: &'a Message) -> Result<(), Error>;
}

pub struct PinWriter<DCC, US> {
    pin_dcc: DCC,
    delay: US,
}

impl<DCC, US> PinWriter<DCC, US>
where
    DCC: ToggleableOutputPin,
    US: DelayUs,
{
    pub fn new(pin_dcc: DCC, delay: US) -> Self {
        Self { pin_dcc, delay }
    }

    async fn write_bit(&mut self, write_one: bool) -> Result<(), Error> {
        trace!("writin bit {}", write_one as i8);
        let us = if write_one {
            ONE_HALF_BIT
        } else {
            ZERO_HALF_BIT
        };
        self.delay
            .delay_us(us)
            .await
            .map_err(|_| Error::DelayError)?;
        self.pin_dcc.toggle().map_err(|_| Error::IOError)?;
        self.delay
            .delay_us(us)
            .await
            .map_err(|_| Error::DelayError)?;
        self.pin_dcc.toggle().map_err(|_| Error::IOError)?;
        Ok(())
    }

    async fn write_preamble(&mut self) -> Result<(), Error> {
        debug!("writing preamble");
        for _ in 0..PREAMBLE_SIZE {
            self.write_bit(true).await?;
        }
        Ok(())
    }
}

impl<DCC, US> Writer for PinWriter<DCC, US>
where
    DCC: ToggleableOutputPin,
    US: DelayUs,
{
    async fn write<'a>(&mut self, msg: &'a Message) -> Result<(), Error> {
        let mut buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
        let bytes_to_write = msg.to_buf(&mut buf);
        self.write_preamble().await?;
        for b in &buf[..bytes_to_write] {
            debug!("writing byte {:x?}", b);
            for position in 0..8 {
                let bit = (b >> position & 0x01) == 1;
                self.write_bit(bit).await?;
            }
        }
        debug!("finish packet");
        self.write_bit(false).await?;
        Ok(())
    }
}
