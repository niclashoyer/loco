#![cfg_attr(not(feature = "std"), no_std)]

pub mod message;
pub mod receiver;
pub mod sender;

/// Errors returned from a SUSI receiver or sender
#[derive(Debug, PartialEq)]
pub enum Error {
	IOError,
	TimerError,
}
