pub mod address;
pub mod direction;
pub mod function;
pub mod message;
pub mod reader;
pub mod speed;
pub mod writer;

#[derive(Debug, PartialEq)]
pub enum Error {
    IOError,
    TimerError,
}
