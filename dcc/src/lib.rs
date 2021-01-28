pub mod direction;
pub mod function;
pub mod message;
pub mod reader;
pub mod speed;
pub mod writer;

pub enum Error {
	IOError,
	TimerError,
}
