use bitvec::prelude::*;
use heapless::Vec;
use loco_core::address::Address;
use loco_core::drive::{Direction, Speed};
use loco_core::functions::*;
use loco_dcc::{
    message::Message,
    writer::{Encoder, Writer},
};
use log::trace;
use num_traits::cast::ToPrimitive;

pub mod togglepins;

#[derive(Debug)]
pub struct Loco {
    addr: Address,
    direction: Direction,
    speed: Speed,
    functions: BitArr!(for 68, in Msb0, u8),
}

impl Loco {
    pub fn new<A: Into<Address>>(addr: A) -> Loco {
        Loco {
            addr: addr.into(),
            direction: Direction::Forward,
            speed: Speed::Stop,
            functions: bitarr![Msb0, u8; 0; 68],
        }
    }

    pub fn is_function_set(&self, func: Function) -> bool {
        self.functions[func.to_usize().unwrap()]
    }

    pub fn set_function(&mut self, func: Function, val: bool) {
        self.functions.set(func.to_usize().unwrap(), val);
    }

    pub fn set_direction(&mut self, direction: Direction) {
        self.direction = direction;
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn set_speed(&mut self, speed: Speed) {
        self.speed = speed;
    }

    pub fn speed(&self) -> Speed {
        self.speed
    }
}

pub struct Station<E: Encoder, const N: usize> {
    locos: Vec<Loco, N>,
    writer: Writer<E>,
    msg: Option<Message>,
    index: usize,
}

impl<E: Encoder, const N: usize> Station<E, N> {
    pub fn new(encoder: E) -> Self {
        Self {
            locos: Vec::new(),
            writer: Writer::new(encoder),
            msg: None,
            index: 0,
        }
    }

    pub fn add_loco(&mut self, addr: Address) {
        let loco = Loco::new(addr);
        self.locos.push(loco).unwrap(); // FIXME: unwrap
    }

    pub fn loco_set_function(&mut self, addr: Address, func: Function, val: bool) {
        for loco in &mut self.locos {
            if loco.addr == addr {
                loco.set_function(func, val);
            }
        }
    }

    pub fn loco_set_drive(&mut self, addr: Address, speed: Speed, direction: Direction) {
        for loco in &mut self.locos {
            if loco.addr == addr {
                loco.set_speed(speed);
                loco.set_direction(direction);
            }
        }
    }

    pub fn run(&mut self) -> nb::Result<(), core::convert::Infallible> {
        if let Some(msg) = &self.msg {
            let ret = self.writer.write(msg);
            if ret.is_ok() {
                self.msg = None;
                Err(nb::Error::WouldBlock)
            } else {
                Err(nb::Error::WouldBlock) // FIXME handle error from dcc
            }
        } else {
            if self.index >= self.locos.len() {
                self.index = 0;
            }
            let loco = self.locos.get(self.index).unwrap();
            self.msg = Some(Message::Drive(loco.addr, loco.direction, loco.speed));
            self.index += 1;
            trace!("{:?}", self.msg);
            Err(nb::Error::WouldBlock)
        }
    }
}
